//! Gravitational lensing screen-space post-process (S2.12).
//!
//! # Pipeline
//! 1. CPU projects lens world positions → screen UV via view-projection matrix
//! 2. Upload lens data (UV + mass) to storage buffer
//! 3. Fullscreen-triangle pass: for each pixel, sum deflection from all lenses,
//!    re-sample HDR texture at distorted UV.

use bevy_ecs::prelude::*;

/// Visual settings for gravitational lensing.
#[derive(Resource, Clone, Copy)]
pub struct LensingSettings {
    pub enabled: bool,
    pub strength: f32,
    pub mass_scale: f32,
}

impl Default for LensingSettings {
    fn default() -> Self {
        Self { enabled: true, strength: 1.0, mass_scale: 5e-25 }
    }
}

// ── GPU data structures ────────────────────────────────────────────────────

const MAX_LENSES: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LensGpuData {
    uv: [f32; 2],
    mass: f32,
    _pad: u32,
}

impl LensGpuData {
    pub fn new(uv: [f32; 2], mass: f32) -> Self {
        Self { uv, mass, _pad: 0 }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LensingUniformsRaw {
    data: [f32; 4], // [count, strength, mass_scale, _pad]
}

// ── Offscreen texture for lensed HDR ───────────────────────────────────────

pub struct LensingTextures {
    pub lensed_texture: wgpu::Texture,
    pub lensed_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl LensingTextures {
    pub fn new(device: &wgpu::Device, hdr_format: wgpu::TextureFormat, width: u32, height: u32) -> Self {
        let lensed_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("lensed_texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: hdr_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let lensed_view = lensed_texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { lensed_texture, lensed_view, width, height }
    }

    pub fn recreate(&mut self, device: &wgpu::Device, hdr_format: wgpu::TextureFormat, width: u32, height: u32) {
        *self = Self::new(device, hdr_format, width, height);
    }
}

// ── Render pipeline ───────────────────────────────────────────────────────

pub struct LensingPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    lens_buffer: wgpu::Buffer,
}

impl LensingPipeline {
    pub fn new(
        device: &wgpu::Device,
        hdr_format: wgpu::TextureFormat,
        hdr_view: &wgpu::TextureView,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lensing_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                include_str!("lensing.wgsl"),
            )),
        });

        // ── Sampler ──────────────────────────────────────────────────
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lensing_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        // ── Uniform buffer ───────────────────────────────────────────
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lensing_uniform_buffer"),
            size: std::mem::size_of::<LensingUniformsRaw>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── Lens storage buffer ──────────────────────────────────────
        let lens_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lensing_lens_buffer"),
            size: (std::mem::size_of::<LensGpuData>() * MAX_LENSES) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── Bind group layout ─────────────────────────────────────────
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lensing_bind_group_layout"),
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
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
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

        // ── Pipeline layout ───────────────────────────────────────────
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lensing_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        // ── Render pipeline ───────────────────────────────────────────
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lensing_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("fullscreen_vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("lensing_fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: hdr_format,
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
        });

        // ── Bind group ────────────────────────────────────────────────
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lensing_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(hdr_view) },
                wgpu::BindGroupEntry { binding: 2, resource: lens_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        Self { pipeline, bind_group_layout, bind_group, sampler, uniform_buffer, lens_buffer }
    }

    /// Rebuild bind group (e.g. after resize — new HDR view).
    pub fn rebind(&mut self, device: &wgpu::Device, hdr_view: &wgpu::TextureView) {
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lensing_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(hdr_view) },
                wgpu::BindGroupEntry { binding: 2, resource: self.lens_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: self.uniform_buffer.as_entire_binding() },
            ],
        });
    }

    /// Upload lens data and uniforms, then execute the lensing pass.
    ///
    /// Always runs the fullscreen pass. When `enabled` is false or `lenses` is empty,
    /// uses count=0 so the shader acts as a passthrough (copy hdr → lensed).
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        settings: &LensingSettings,
        lenses: &[LensGpuData],
        lensed_view: &wgpu::TextureView,
    ) {
        let count = if settings.enabled {
            lenses.len().min(MAX_LENSES) as u32
        } else {
            0
        };

        // Upload lens data (unused when count=0, but written for simplicity)
        if !lenses.is_empty() {
            queue.write_buffer(&self.lens_buffer, 0, bytemuck::cast_slice(lenses));
        }

        // Upload uniforms
        let raw = LensingUniformsRaw {
            data: [count as f32, settings.strength, settings.mass_scale, 0.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&raw));

        // Render fullscreen triangle (passthrough when count=0)
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("lensing_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: lensed_view,
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
