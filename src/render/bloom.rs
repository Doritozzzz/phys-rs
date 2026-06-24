//! Bloom post-processing — HDR extract, separable gaussian blur, ACES tonemap composite.
//!
//! # Pipeline
//! 1. Extract bright pixels from HDR → half-res bloom texture
//! 2. Gaussian blur horizontal (9-tap) → ping texture
//! 3. Gaussian blur vertical (9-tap) → bloom texture
//! 4. Composite HDR + bloom → ACES tonemap → swapchain

use bevy_ecs::prelude::*;

/// RGB multipliers for hot stars to produce HDR values > 1.0.
#[derive(Resource)]
pub struct BloomSettings {
    pub enabled: bool,
    pub threshold: f32,
    pub intensity: f32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self { enabled: true, threshold: 0.8, intensity: 0.5 }
    }
}

// ── Offscreen textures ─────────────────────────────────────────────────────

pub struct BloomTextures {
    pub hdr_texture: wgpu::Texture,
    pub hdr_view: wgpu::TextureView,
    pub bloom_texture: wgpu::Texture,
    pub bloom_view: wgpu::TextureView,
    pub ping_texture: wgpu::Texture,
    pub ping_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl BloomTextures {
    pub fn new(device: &wgpu::Device, hdr_format: wgpu::TextureFormat, width: u32, height: u32) -> Self {
        let hdr_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hdr_texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: hdr_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let hdr_view = hdr_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bw = (width + 1) / 2;
        let bh = (height + 1) / 2;

        let bloom_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloom_texture"),
            size: wgpu::Extent3d { width: bw, height: bh, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: hdr_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let bloom_view = bloom_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let ping_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloom_ping_texture"),
            size: wgpu::Extent3d { width: bw, height: bh, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: hdr_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let ping_view = ping_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { hdr_texture, hdr_view, bloom_texture, bloom_view, ping_texture, ping_view, width, height }
    }

    pub fn recreate(&mut self, device: &wgpu::Device, hdr_format: wgpu::TextureFormat, width: u32, height: u32) {
        *self = Self::new(device, hdr_format, width, height);
    }
}

// ── GPU-side uniform layout ────────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BloomUniformsRaw {
    threshold: f32,
    intensity: f32,
}

// ── Render pipelines for bloom passes ──────────────────────────────────────

pub struct BloomPipeline {
    extract_pipeline: wgpu::RenderPipeline,
    blur_h_pipeline: wgpu::RenderPipeline,
    blur_v_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    passthrough_pipeline: wgpu::RenderPipeline,

    extract_layout: wgpu::BindGroupLayout,
    blur_layout: wgpu::BindGroupLayout,
    composite_layout: wgpu::BindGroupLayout,
    passthrough_layout: wgpu::BindGroupLayout,

    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,

    // Bind groups — rebuilt via rebind() on resize
    extract_bind_group: wgpu::BindGroup,
    blur_h_bind_group: wgpu::BindGroup,
    blur_v_bind_group: wgpu::BindGroup,
    composite_bind_group: wgpu::BindGroup,
    passthrough_bind_group: wgpu::BindGroup,
}

impl BloomPipeline {
    pub fn new(
        device: &wgpu::Device,
        hdr_format: wgpu::TextureFormat,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                include_str!("bloom.wgsl"),
            )),
        });

        // ── Sampler (point/linear for bloom) ────────────────────────
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        // ── Uniform buffer ──────────────────────────────────────────
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom_uniform_buffer"),
            size: std::mem::size_of::<BloomUniformsRaw>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── Bind group layouts ──────────────────────────────────────

        // Group 0: Extract — sampler + hdr_texture + uniform
        let extract_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_extract_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Group 1: Blur — sampler + texture source
        let blur_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_blur_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        // Group 2: Composite — 1 sampler + 2 textures + uniform
        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_composite_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // ── Passthrough layout (sampler + texture, no uniform) ────
        let passthrough_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_passthrough_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        // ── Pipeline layouts ────────────────────────────────────────
        let layout_extract = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom_extract_pipeline_layout"),
            bind_group_layouts: &[Some(&extract_layout)],
            immediate_size: 0,
        });
        let layout_blur = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom_blur_pipeline_layout"),
            bind_group_layouts: &[Some(&blur_layout)],
            immediate_size: 0,
        });
        let layout_composite = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom_composite_pipeline_layout"),
            bind_group_layouts: &[Some(&composite_layout)],
            immediate_size: 0,
        });
        let layout_passthrough = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom_passthrough_pipeline_layout"),
            bind_group_layouts: &[Some(&passthrough_layout)],
            immediate_size: 0,
        });

        // ── Helper: create a fullscreen-tri pipeline ────────────────
        let make_pipeline = |device: &wgpu::Device,
                             layout: &wgpu::PipelineLayout,
                             entry: &str,
                             format: wgpu::TextureFormat|
         -> wgpu::RenderPipeline {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("fullscreen_vs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let extract_pipeline = make_pipeline(device, &layout_extract, "bloom_extract_fs", hdr_format);
        let blur_h_pipeline = make_pipeline(device, &layout_blur, "gaussian_blur_h_fs", hdr_format);
        let blur_v_pipeline = make_pipeline(device, &layout_blur, "gaussian_blur_v_fs", hdr_format);
        let composite_pipeline = make_pipeline(device, &layout_composite, "composite_fs", surface_format);
        let passthrough_pipeline = make_pipeline(device, &layout_passthrough, "passthrough_fs", surface_format);

        // Dummy bind groups (will be rebuilt by rebind())
        let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloom_dummy"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: hdr_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dummy_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let make_dummy_bg = |device: &wgpu::Device,
                             layout: &wgpu::BindGroupLayout,
                             view: &wgpu::TextureView,
                             sampler: &wgpu::Sampler|
         -> wgpu::BindGroup {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom_dummy_bg"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(sampler) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(view) },
                ],
            })
        };

        let extract_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_dummy_extract_bg"),
            layout: &extract_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&dummy_view) },
                wgpu::BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        let blur_h_bg = make_dummy_bg(device, &blur_layout, &dummy_view, &sampler);
        let blur_v_bg = make_dummy_bg(device, &blur_layout, &dummy_view, &sampler);

        let composite_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_dummy_composite_bg"),
            layout: &composite_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&dummy_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&dummy_view) },
                wgpu::BindGroupEntry { binding: 3, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        let passthrough_bg = make_dummy_bg(device, &passthrough_layout, &dummy_view, &sampler);

        Self {
            extract_pipeline,
            blur_h_pipeline,
            blur_v_pipeline,
            composite_pipeline,
            passthrough_pipeline,
            extract_layout,
            blur_layout,
            composite_layout,
            passthrough_layout,
            sampler,
            uniform_buffer,
            extract_bind_group: extract_bg,
            blur_h_bind_group: blur_h_bg,
            blur_v_bind_group: blur_v_bg,
            composite_bind_group: composite_bg,
            passthrough_bind_group: passthrough_bg,
        }
    }

    /// Rebuild bind groups after resize.
    pub fn rebind(
        &mut self,
        device: &wgpu::Device,
        hdr_view: &wgpu::TextureView,
        bloom_view: &wgpu::TextureView,
        ping_view: &wgpu::TextureView,
    ) {
        self.extract_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_extract_bg"),
            layout: &self.extract_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(hdr_view) },
                wgpu::BindGroupEntry { binding: 2, resource: self.uniform_buffer.as_entire_binding() },
            ],
        });

        self.blur_h_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_blur_h_bg"),
            layout: &self.blur_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(bloom_view) },
            ],
        });

        self.blur_v_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_blur_v_bg"),
            layout: &self.blur_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(ping_view) },
            ],
        });

        self.composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_composite_bg"),
            layout: &self.composite_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(hdr_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(bloom_view) },
                wgpu::BindGroupEntry { binding: 3, resource: self.uniform_buffer.as_entire_binding() },
            ],
        });

        self.passthrough_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_passthrough_bg"),
            layout: &self.passthrough_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(hdr_view) },
            ],
        });
    }

    /// Update bloom threshold and intensity uniforms.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, threshold: f32, intensity: f32) {
        let raw = BloomUniformsRaw { threshold, intensity };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&raw));
    }

    /// Execute all bloom passes (1 encoder, single command buffer).
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bloom_view: &wgpu::TextureView,
        ping_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    ) {
        // ── Pass 1: Extract bright pixels → bloom ─────────────────
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_extract"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: bloom_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.extract_pipeline);
            pass.set_bind_group(0, &self.extract_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── Pass 2: Gaussian blur horizontal (bloom → ping) ──────
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_blur_h"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: ping_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.blur_h_pipeline);
            pass.set_bind_group(0, &self.blur_h_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── Pass 3: Gaussian blur vertical (ping → bloom) ────────
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_blur_v"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: bloom_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.blur_v_pipeline);
            pass.set_bind_group(0, &self.blur_v_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── Pass 4: Composite HDR + bloom → ACES → swapchain ──────
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &self.composite_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// Debug: sample HDR texture and write to swapchain (no bloom, no ACES).
    /// If this shows content, the HDR pipeline is working but ACES/composite is wrong.
    pub fn render_passthrough(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bloom_passthrough"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            multiview_mask: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.passthrough_pipeline);
        pass.set_bind_group(0, &self.passthrough_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
