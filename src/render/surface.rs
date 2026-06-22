//! wgpu render surface, device, queue, pipeline, and frame presentation.

use std::f64::consts::FRAC_PI_4;
use bevy_ecs::prelude::*;
use glam::{DMat4, DVec3};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use super::render_pipeline::{
    create_viewproj_bind_group_layout,
    InstanceData, LineVertex, SpherePipeline, LinePipeline, SkyboxPipeline,
};
use super::camera::CameraState;
use super::{RenderScaleMode, ShowVelocityVectors, ShowForceVectors, ShowSectorGrid};
use crate::components::{
    BodyType, BoundingRadius, Mass, Temperature,
    OrbitTrail, Velocity, Force,
    SmoothedDensity, Pressure,
};
use crate::core::coordinates::{LocalPosition, Sector};
use crate::core::config::UniverseConfig;

/// Owns the wgpu device, queue, surface, configuration, and render pipeline.
///
/// # Safety
/// The `Surface` inside this struct is transmuted to `'static` because it
/// borrows from a window held in `super::App::window` (an `Arc<Window>`).
/// `RenderContext` must be dropped **before** the window it borrows from.
pub struct RenderContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub sphere_pipeline: SpherePipeline,
    pub line_pipeline: LinePipeline,
    pub skybox_pipeline: SkyboxPipeline,
    pub viewproj_layout: wgpu::BindGroupLayout,
}

impl RenderContext {
    /// Create a new RenderContext from a window. Blocking (uses `pollster`).
    ///
    /// # Safety
    /// The returned `Surface` is transmuted to `'static`. Caller must ensure
    /// the window outlives this `RenderContext`.
    pub fn new(window: &Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window).expect("Failed to create wgpu surface");

        // SAFETY: the window is owned by App::window (Arc<Window>), which
        // outlives this RenderContext. RenderContext::drop runs before
        // App::window is dropped (enforced by field order and Drop impl).
        let surface: wgpu::Surface<'static> = unsafe { std::mem::transmute(surface) };

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find a compatible GPU adapter");

        let (device, queue) =
            pollster::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("phys_rs_render_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                    experimental_features: Default::default(),
                    trace: Default::default(),
                },
            ))
            .expect("Failed to create wgpu device");

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let viewproj_layout = create_viewproj_bind_group_layout(&device);
        let sphere_pipeline = SpherePipeline::new(&device, &config, &queue, &viewproj_layout);
        let line_pipeline = LinePipeline::new(&device, &config, &viewproj_layout);
        let skybox_pipeline = SkyboxPipeline::new(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            sphere_pipeline,
            line_pipeline,
            skybox_pipeline,
            viewproj_layout,
        }
    }

    /// Reconfigure the surface after a window resize.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.sphere_pipeline.resize_depth(&self.device, size.width, size.height);
    }

    /// Acquire frame texture, build instance data from ECS, render, and present.
    pub fn present_frame(&self, world: &mut World) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return,
            wgpu::CurrentSurfaceTexture::Outdated => return,
            wgpu::CurrentSurfaceTexture::Lost => {
                panic!("wgpu surface lost");
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // ── Build view-projection matrix from CameraState ──────────
        let cam_pos;
        let (view_proj, inv_view_proj, ss) = {
            let cam = world.get_resource::<CameraState>();
            let config = world.get_resource::<UniverseConfig>();
            match (cam, config) {
                (Some(cam), Some(config)) => {
                    let aspect = self.config.width as f64 / self.config.height as f64;
                    let proj = DMat4::perspective_rh(FRAC_PI_4 * 0.5, aspect, 0.1, 1.0e15);
                    let vw = DMat4::look_at_rh(cam.position, cam.position + cam.forward, cam.up);
                    let vp = proj * vw;
                    let ivp = vp.inverse();
                    (vp, ivp, config.sector_size)
                }
                _ => (DMat4::IDENTITY, DMat4::IDENTITY, 1e12),
            }
        };

        let view_proj_f32: glam::Mat4 = glam::Mat4::from_cols(
            view_proj.x_axis.as_vec4().into(),
            view_proj.y_axis.as_vec4().into(),
            view_proj.z_axis.as_vec4().into(),
            view_proj.w_axis.as_vec4().into(),
        );

        let inv_view_proj_f32: glam::Mat4 = glam::Mat4::from_cols(
            inv_view_proj.x_axis.as_vec4().into(),
            inv_view_proj.y_axis.as_vec4().into(),
            inv_view_proj.z_axis.as_vec4().into(),
            inv_view_proj.w_axis.as_vec4().into(),
        );

        cam_pos = world.get_resource::<CameraState>()
            .map(|c| c.position)
            .unwrap_or(DVec3::ZERO);

        // ── Read rendering resources ───────────────────────────────
        let scale_mode = world.get_resource::<RenderScaleMode>()
            .copied()
            .unwrap_or(RenderScaleMode::Linear);

        let show_velocity = world.get_resource::<ShowVelocityVectors>()
            .copied()
            .unwrap_or(ShowVelocityVectors(false));

        let show_force = world.get_resource::<ShowForceVectors>()
            .copied()
            .unwrap_or(ShowForceVectors(false));

        let show_grid = world.get_resource::<ShowSectorGrid>()
            .copied()
            .unwrap_or(ShowSectorGrid(false));

        // Trail lines collected from OrbitTrail components below

        let _selected = world.get_resource::<super::SelectedEntity>()
            .map(|s| s.0);

        // ── Collect instance data from ECS ─────────────────────────
        let mut instances: Vec<InstanceData> = Vec::new();

        let mut entity_query = world.query::<(
            &Sector, &LocalPosition, &BoundingRadius, &BodyType,
            Option<&Temperature>,
            Option<&Mass>,
            Option<&SmoothedDensity>,
            Option<&Pressure>,
            Option<&Velocity>,
            Option<&Force>,
        )>();

        // Also gather line data: trails, velocity vectors, force vectors, grid
        let mut line_vertices: Vec<LineVertex> = Vec::new();

        for (sector, local, radius, body_type, temp, mass, _density, _pressure, _vel, _force) in
            entity_query.iter(world)
        {
            let world_pos = DVec3::new(
                sector.0.x as f64 * ss,
                sector.0.y as f64 * ss,
                sector.0.z as f64 * ss,
            ) + local.0;

            let relative = world_pos - cam_pos;

            // ── Determine color ────────────────────────────────────
            let color = if *body_type == BodyType::FluidParticle {
                // SPH density-based color ramp (S2.9)
                let density = _density.map(|d| d.0).unwrap_or(0.0);
                sph_density_color(density)
            } else {
                temp
                    .map(|t| temperature_to_rgb(t.0))
                    .unwrap_or(match body_type {
                        BodyType::BlackHole => [0.0, 0.0, 0.0, 1.0],
                        BodyType::Star => [0.5, 0.5, 0.2, 1.0],
                        _ => [0.4, 0.4, 0.5, 1.0],
                    })
            };

            // ── Determine scale ────────────────────────────────────
            let scale = match scale_mode {
                RenderScaleMode::Linear => radius.0,
                RenderScaleMode::Logarithmic => {
                    let m = mass.map(|m| m.0).unwrap_or(radius.0 * radius.0 * radius.0 * 1000.0);
                    let r_base = 5.0_f64;
                    let k = 15.0_f64;
                    let m_ref = 1e22_f64;
                    r_base + k * (1.0 + m / m_ref).log10()
                }
            };

            instances.push(InstanceData {
                position: [relative.x as f32, relative.y as f32, relative.z as f32],
                scale: scale as f32,
                color,
            });

            // ── Trail lines (OrbitTrail) ──────────────────────────
            // Already collected in TrailLines resource, but we also
            // collect from OrbitTrail component directly.
        }

        // ── Collect OrbitTrail component lines ─────────────────────
        // Walk entities with OrbitTrail and convert to line segments
        let mut trail_query = world.query::<(&Sector, &LocalPosition, &OrbitTrail)>();
        for (_sector, _local, trail) in trail_query.iter(world) {
            let entries: Vec<_> = trail.history.iter().collect();
            let max_i = entries.len().max(1) as f32;
            for i in 1..entries.len() {
                let (sec_a, loc_a) = entries[i - 1];
                let (sec_b, loc_b) = entries[i];
                let pa = DVec3::new(
                    sec_a.0.x as f64 * ss,
                    sec_a.0.y as f64 * ss,
                    sec_a.0.z as f64 * ss,
                ) + loc_a.0 - cam_pos;
                let pb = DVec3::new(
                    sec_b.0.x as f64 * ss,
                    sec_b.0.y as f64 * ss,
                    sec_b.0.z as f64 * ss,
                ) + loc_b.0 - cam_pos;
                let alpha = (1.0 - i as f32 / max_i) * 0.7;
                line_vertices.push(LineVertex {
                    position: [pa.x as f32, pa.y as f32, pa.z as f32],
                    color: [0.4, 0.7, 1.0, alpha],
                });
                line_vertices.push(LineVertex {
                    position: [pb.x as f32, pb.y as f32, pb.z as f32],
                    color: [0.4, 0.7, 1.0, alpha],
                });
            }
        }

        // ── Velocity vectors (selected or all if toggled) ──────────
        if show_velocity.0 {
            let mut vel_query = world.query::<(&Sector, &LocalPosition, &Velocity)>();
            for (sector, local, velocity) in vel_query.iter(world) {
                let world_pos = DVec3::new(
                    sector.0.x as f64 * ss,
                    sector.0.y as f64 * ss,
                    sector.0.z as f64 * ss,
                ) + local.0;
                let p = world_pos - cam_pos;
                let dir = velocity.0;
                let mag = dir.length();
                if mag > 1e-6 {
                    let len = mag.max(10.0).min(1e9);
                    let tip = p + dir / mag * len;
                    line_vertices.push(LineVertex {
                        position: [p.x as f32, p.y as f32, p.z as f32],
                        color: [0.0, 1.0, 0.0, 0.8],
                    });
                    line_vertices.push(LineVertex {
                        position: [tip.x as f32, tip.y as f32, tip.z as f32],
                        color: [0.0, 1.0, 0.0, 0.8],
                    });
                }
            }
        }

        // ── Force vectors ──────────────────────────────────────────
        if show_force.0 {
            let mut force_query = world.query::<(&Sector, &LocalPosition, &Force)>();
            for (sector, local, force) in force_query.iter(world) {
                let world_pos = DVec3::new(
                    sector.0.x as f64 * ss,
                    sector.0.y as f64 * ss,
                    sector.0.z as f64 * ss,
                ) + local.0;
                let p = world_pos - cam_pos;
                let dir = force.0;
                let mag = dir.length();
                if mag > 1e-6 {
                    let len = (mag * 0.001).max(10.0).min(1e9);
                    let tip = p + dir / mag * len;
                    line_vertices.push(LineVertex {
                        position: [p.x as f32, p.y as f32, p.z as f32],
                        color: [1.0, 0.6, 0.0, 0.8],
                    });
                    line_vertices.push(LineVertex {
                        position: [tip.x as f32, tip.y as f32, tip.z as f32],
                        color: [1.0, 0.6, 0.0, 0.8],
                    });
                }
            }
        }

        // ── Sector wireframe grid ──────────────────────────────────
        if show_grid.0 {
            let s = ss as f32;
            // Draw edges of camera sector and its 26 neighbors
            let cam_sector = world.get_resource::<CameraState>()
                .and_then(|cam| {
                    // Find which sector the camera is in
                    let cx = cam.position.x;
                    let cy = cam.position.y;
                    let cz = cam.position.z;
                    Some(Sector(glam::I64Vec3::new(
                        (cx / ss).floor() as i64,
                        (cy / ss).floor() as i64,
                        (cz / ss).floor() as i64,
                    )))
                })
                .unwrap_or(Sector(glam::I64Vec3::ZERO));

            for dx in -1i64..=1 {
                for dy in -1i64..=1 {
                    for dz in -1i64..=1 {
                        let ox = (cam_sector.0.x + dx) as f32 * s;
                        let oy = (cam_sector.0.y + dy) as f32 * s;
                        let oz = (cam_sector.0.z + dz) as f32 * s;
                        // Only draw near the camera
                        let dist = (DVec3::new(ox as f64, oy as f64, oz as f64) - cam_pos).length();
                        if dist > ss * 3.0 { continue; }

                        // Relative to camera
                        let rx = ox - cam_pos.x as f32;
                        let ry = oy - cam_pos.y as f32;
                        let rz = oz - cam_pos.z as f32;

                        let grid_color = [0.0, 0.4, 0.8, 0.15];
                        // 12 edges of a box
                        let corners = [
                            [rx, ry, rz], [rx + s, ry, rz], [rx, ry + s, rz], [rx + s, ry + s, rz],
                            [rx, ry, rz + s], [rx + s, ry, rz + s], [rx, ry + s, rz + s], [rx + s, ry + s, rz + s],
                        ];
                        let edges: [(usize, usize); 12] = [
                            (0,1),(1,3),(3,2),(2,0),
                            (4,5),(5,7),(7,6),(6,4),
                            (0,4),(1,5),(2,6),(3,7),
                        ];
                        for (i, j) in edges {
                            line_vertices.push(LineVertex { position: corners[i], color: grid_color });
                            line_vertices.push(LineVertex { position: corners[j], color: grid_color });
                        }
                    }
                }
            }
        }

        // ── Upload uniforms ───────────────────────────────────────
        self.sphere_pipeline.update_uniform(&self.queue, &view_proj_f32);
        self.line_pipeline.update_uniform(&self.queue, &view_proj_f32);
        self.skybox_pipeline.update_uniform(&self.queue, &inv_view_proj_f32);

        // ── Upload instance and line data ─────────────────────────
        self.sphere_pipeline.update_instances(&self.queue, &instances);
        self.line_pipeline.update_vertices(&self.queue, &line_vertices);

        // ── Encode and submit render pass ─────────────────────────
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        // Pass 1: Skybox (depth off) + Spheres (depth on) + Lines (depth on, write off)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.sphere_pipeline.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // 1a. Skybox (no depth test/write)
            self.skybox_pipeline.render(&mut pass);

            // 1b. Spheres
            pass.set_pipeline(&self.sphere_pipeline.pipeline);
            pass.set_bind_group(0, &self.sphere_pipeline.bind_group, &[]);
            pass.set_vertex_buffer(0, self.sphere_pipeline.sphere_mesh.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.sphere_pipeline.instance_buffer.slice(..));
            pass.set_index_buffer(
                self.sphere_pipeline.sphere_mesh.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(
                0..self.sphere_pipeline.sphere_mesh.index_count,
                0,
                0..(instances.len() as u32).min(10_000),
            );

            // 1c. Lines
            if !line_vertices.is_empty() {
                self.line_pipeline.render(&mut pass, line_vertices.len() as u32);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

/// Approximate blackbody color from temperature (Wien's law approximation).
fn temperature_to_rgb(temp_kelvin: f64) -> [f32; 4] {
    let t = temp_kelvin / 100.0;
    let (r, g, b) = if t <= 66.0 {
        let r = 255.0;
        let g = 99.470 * t.ln() - 161.120;
        let b = if t <= 19.0 { 0.0 } else { 138.520 * t.ln() - 305.040 };
        (r, g, b)
    } else {
        let r = 329.700 * (t - 60.0).powf(-0.133);
        let g = 288.120 * (t - 60.0).powf(-0.0755);
        let b = 255.0;
        (r, g, b)
    };
    [(r / 255.0) as f32, (g / 255.0) as f32, (b / 255.0) as f32, 1.0]
}

/// SPH density-based color ramp (S2.9).
///
/// Low density → dark blue, medium → cyan/green, high → red/white.
fn sph_density_color(density: f64) -> [f32; 4] {
    let d = (density * 0.1).clamp(0.0, 1.0) as f32; // normalize, tune as needed
    if d < 0.33 {
        let t = d / 0.33;
        [0.1 * (1.0 - t) + 0.2 * t, 0.2 * (1.0 - t) + 0.5 * t, 0.6, 0.5 + t * 0.3]
    } else if d < 0.66 {
        let t = (d - 0.33) / 0.33;
        [0.2 * (1.0 - t) + 0.8 * t, 0.5 * (1.0 - t) + 0.6 * t, 0.6 * (1.0 - t) + 0.2 * t, 0.8]
    } else {
        let t = (d - 0.66) / 0.34;
        [0.8 * (1.0 - t) + 1.0 * t, 0.6 * (1.0 - t) + 0.8 * t, 0.2 * (1.0 - t), 0.8 + t * 0.2]
    }
}
