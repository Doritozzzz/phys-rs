//! wgpu render surface, device, queue, pipeline, and frame presentation.

use bevy_ecs::prelude::*;
use glam::{DMat4, DVec2, DVec3, DVec4};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use std::sync::Arc;

use super::bloom::{BloomPipeline, BloomSettings, BloomTextures};
use super::lensing::{LensingPipeline, LensingSettings, LensingTextures, LensGpuData};
use super::render_pipeline::{
    create_viewproj_bind_group_layout,
    InstanceData, LineVertex, SpherePipeline, LinePipeline, SkyboxPipeline,
};
use super::camera::CameraState;
use super::ui::EntityHudData;
use super::{RenderScaleMode, ShowVelocityVectors, ShowForceVectors, ShowSectorGrid, ShowTrails, RenderDirectToSwapchain, SelectedEntity};
use crate::components::lensing::GravitationalLens;
use crate::components::{
    BodyType, BoundingRadius, Mass, Temperature,
    OrbitTrail, Velocity, Force,
    SmoothedDensity, Pressure, EntityName,
};
use crate::core::coordinates::{LocalPosition, Sector};
use crate::core::config::UniverseConfig;
use crate::physics::kepler_orbit_points;

/// Projection FOV in radians (same as DMat4::perspective_rh below).
const FOV_RAD: f64 = std::f64::consts::FRAC_PI_4 * 0.5;

/// Convert target screen-space pixels to world-space length at cam_distance.
fn screen_space_length(cam_distance: f64, screen_height: f64, target_pixels: f64) -> f64 {
    let pixel_world = 2.0 * (FOV_RAD * 0.5).tan() * cam_distance / screen_height;
    target_pixels * pixel_world
}

/// Choose best HDR format: prefers Rgba16Float if it supports RENDER_ATTACHMENT
/// + TEXTURE_BINDING + filterable float sampling. Falls back to Rgba32Float.
fn choose_hdr_format(adapter: &wgpu::Adapter) -> wgpu::TextureFormat {
    let needed_usages =
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    let candidates = [wgpu::TextureFormat::Rgba16Float, wgpu::TextureFormat::Rgba32Float];
    for &fmt in &candidates {
        let f = adapter.get_texture_format_features(fmt);
        let usages_ok = f.allowed_usages.contains(needed_usages);
        let filterable_ok = f.flags.contains(wgpu::TextureFormatFeatureFlags::FILTERABLE);
        if usages_ok && filterable_ok {
            return fmt;
        }
    }
    wgpu::TextureFormat::Rgba32Float
}

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
    pub bloom_textures: BloomTextures,
    pub bloom_pipeline: BloomPipeline,
    pub lensing_textures: LensingTextures,
    pub lensing_pipeline: LensingPipeline,
    hdr_format: wgpu::TextureFormat,
    // ── egui UI ──────────────────────────────────────────────────────────────
    pub window: Arc<Window>,
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
    pub ui_state: super::ui::UiState,
}

impl RenderContext {
    /// Create a new RenderContext from a window. Blocking (uses `pollster`).
    ///
    /// # Safety
    /// The returned `Surface` is transmuted to `'static`. Caller must ensure
    /// the window outlives this `RenderContext`.
    pub fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(&*window).expect("Failed to create wgpu surface");

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

        // ── Debug: catch wgpu validation errors ────────────────────
        device.on_uncaptured_error(std::sync::Arc::new(|err: wgpu::Error| {
            eprintln!("[wgpu] *** UNCAPTURED ERROR ***\n{err:#}");
        }));
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

        // ── Determine HDR render target format ──────────────────────
        let hdr_format = choose_hdr_format(&adapter);

        let viewproj_layout = create_viewproj_bind_group_layout(&device);
        let sphere_pipeline = SpherePipeline::new(&device, &config, &queue, &viewproj_layout, hdr_format);
        let line_pipeline = LinePipeline::new(&device, &config, &viewproj_layout, config.format);
        let skybox_pipeline = SkyboxPipeline::new(&device, &config, hdr_format);

        let bloom_textures = BloomTextures::new(&device, hdr_format, config.width, config.height);
        let lensing_textures = LensingTextures::new(&device, hdr_format, config.width, config.height);
        let lensing_pipeline = LensingPipeline::new(
            &device,
            hdr_format,
            &bloom_textures.hdr_view,
        );
        // Bloom reads from lensed texture (lensing passthrough when disabled).
        let mut bloom_pipeline = BloomPipeline::new(&device, hdr_format, config.format);
        bloom_pipeline.rebind(
            &device,
            &lensing_textures.lensed_view,
            &bloom_textures.bloom_view,
            &bloom_textures.ping_view,
        );

        // ── egui UI init ────────────────────────────────────────────
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            None,
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            egui_wgpu::RendererOptions::default(),
        );

        Self {
            surface,
            device,
            queue,
            config,
            sphere_pipeline,
            line_pipeline,
            skybox_pipeline,
            viewproj_layout,
            bloom_textures,
            bloom_pipeline,
            lensing_textures,
            lensing_pipeline,
            hdr_format,
            window,
            egui_ctx,
            egui_state,
            egui_renderer,
            ui_state: super::ui::UiState::default(),
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
        self.bloom_textures.recreate(&self.device, self.hdr_format, size.width, size.height);
        self.lensing_textures.recreate(&self.device, self.hdr_format, size.width, size.height);
        self.lensing_pipeline.rebind(&self.device, &self.bloom_textures.hdr_view);
        // Bloom reads from lensed texture (lensing passthrough when disabled).
        self.bloom_pipeline.rebind(
            &self.device,
            &self.lensing_textures.lensed_view,
            &self.bloom_textures.bloom_view,
            &self.bloom_textures.ping_view,
        );
        let _ = self.egui_state.on_window_event(
            &*self.window,
            &winit::event::WindowEvent::Resized(size),
        );
    }

    /// Acquire frame texture, build instance data from ECS, render, and present.
    pub fn present_frame(&mut self, world: &mut World) {
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

        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let hdr_view = &self.bloom_textures.hdr_view;

        // ── Build view-projection matrix from CameraState ──────────
        let cam_pos;
        let (view_proj, inv_view_proj, ss) = {
            let cam = world.get_resource::<CameraState>();
            let config = world.get_resource::<UniverseConfig>();
            match (cam, config) {
                (Some(cam), Some(config)) => {
                    let aspect = self.config.width as f64 / self.config.height as f64;
                    let proj = DMat4::perspective_rh(FOV_RAD, aspect, 0.1, 1.0e15);
                    // The shader receives positions already relative to the camera (world_pos - cam_pos).
                    // So the view matrix should only apply the camera rotation, not another translation.
                    let view = DMat4::look_at_rh(DVec3::ZERO, cam.forward, cam.up);
                    let vp = proj * view;
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

        let show_trails = world.get_resource::<ShowTrails>()
            .copied()
            .unwrap_or(ShowTrails(true));

        // Trail lines collected from OrbitTrail components below

        // ── Egui UI data: entity list + selected entity HUD ─────────
        let entity_list: Vec<(Entity, String)> = {
            let mut query = world.query::<(Entity, &EntityName)>();
            query.iter(world).map(|(e, n)| (e, n.0.clone())).collect()
        };
        let current_selected = world.get_resource::<SelectedEntity>()
            .map(|s| s.0)
            .unwrap_or(None);

        let hud_data = current_selected.and_then(|entity| {
            world.get_entity(entity).ok().and_then(|e| {
                let name = e.get::<EntityName>().map(|n| n.0.clone()).unwrap_or_default();
                let mass = e.get::<Mass>().map(|m| m.0);
                let velocity = e.get::<Velocity>().map(|v| v.0);
                let temperature = e.get::<Temperature>().map(|t| t.0);
                let body_type = e.get::<BodyType>().copied();
                let position = {
                    let sector = e.get::<Sector>();
                    let local = e.get::<LocalPosition>();
                    match (sector, local) {
                        (Some(s), Some(l)) => Some(DVec3::new(
                            s.0.x as f64 * ss,
                            s.0.y as f64 * ss,
                            s.0.z as f64 * ss,
                        ) + l.0),
                        _ => None,
                    }
                };
                Some(EntityHudData { name, mass, velocity, position, temperature, body_type })
            })
        });

        let direct_to_swapchain = world.get_resource::<RenderDirectToSwapchain>()
            .copied()
            .unwrap_or(RenderDirectToSwapchain(false))
            .0;

        let screen_height = self.config.height as f64;

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

        // ── OrbitTrail lines: predicted Kepler orbit ─────────────────
        if show_trails.0 {
        // Collect massive bodies for central body finding
        let mut mass_query = world.query::<(Entity, &Sector, &LocalPosition, &Mass)>();
        let masses: Vec<(Entity, DVec3, f64)> = mass_query.iter(world)
            .map(|(e, s, l, m)| {
                let pos = DVec3::new(
                    s.0.x as f64 * ss,
                    s.0.y as f64 * ss,
                    s.0.z as f64 * ss,
                ) + l.0;
                (e, pos, m.0)
            })
            .collect();

        let mut trail_query = world.query::<(
            Entity, &Sector, &LocalPosition, &OrbitTrail, Option<&Velocity>,
        )>();
        for (entity, sector, local, trail, vel) in trail_query.iter(world) {
            let world_pos = DVec3::new(
                sector.0.x as f64 * ss,
                sector.0.y as f64 * ss,
                sector.0.z as f64 * ss,
            ) + local.0;

            // Kepler-predicted orbit from current state vector
            if let Some(vel) = vel {
                let most_massive = masses.iter()
                    .filter(|(e, _, _)| *e != entity)
                    .max_by(|(_, _, m1), (_, _, m2)| {
                        m1.partial_cmp(m2).unwrap_or(std::cmp::Ordering::Equal)
                    });
                if let Some((_e, central_pos, central_mass)) = most_massive {
                    let r_rel = world_pos - *central_pos;
                    let pts = kepler_orbit_points(
                        r_rel, vel.0, *central_mass, trail.orbit_point_count,
                    );
                    if !pts.is_empty() {
                        let color = [0.0, 0.8, 1.0, 0.9];
                        for i in 0..pts.len() {
                            let p0 = pts[i] + *central_pos - cam_pos;
                            let p1 = pts[(i + 1) % pts.len()] + *central_pos - cam_pos;
                            line_vertices.push(LineVertex {
                                position: [p0.x as f32, p0.y as f32, p0.z as f32],
                                color,
                            });
                            line_vertices.push(LineVertex {
                                position: [p1.x as f32, p1.y as f32, p1.z as f32],
                                color,
                            });
                        }
                    }
                }
            }
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
                    let cam_dist = p.length().max(1.0);
                    let len = screen_space_length(cam_dist, screen_height, 120.0);
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
                if mag > 1e-8 {
                    let cam_dist = p.length().max(1.0);
                    let len = screen_space_length(cam_dist, screen_height, 120.0);
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

        // ── DIAGNOSTIC: trace line generation (temporary) ────────

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

        // Pass 1: Skybox (depth off) + Spheres (depth on) — HDR target
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: hdr_view,
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
        }

        // ── Gravitational lensing (S2.12) ──────────────────────────
        let lens_settings = world
            .get_resource::<LensingSettings>()
            .copied()
            .unwrap_or(LensingSettings::default());

        let mut lens_data: Vec<LensGpuData> = Vec::new();
        if lens_settings.enabled {
            let mut lens_query = world.query::<(
                &Sector,
                &LocalPosition,
                &Mass,
                &GravitationalLens,
            )>();
            for (sector, local, mass, _lens) in lens_query.iter(world) {
                let world_pos = DVec3::new(
                    sector.0.x as f64 * ss,
                    sector.0.y as f64 * ss,
                    sector.0.z as f64 * ss,
                ) + local.0;
                let clip = view_proj * DVec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
                if clip.w <= 0.0 { continue; }
                let ndc = DVec2::new(clip.x, clip.y) / clip.w;
                let uv = DVec2::new(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5);
                if uv.x >= -0.3 && uv.x <= 1.3 && uv.y >= -0.3 && uv.y <= 1.3 {
                    lens_data.push(LensGpuData::new(
                        [uv.x as f32, uv.y as f32],
                        mass.0 as f32,
                    ));
                    if lens_data.len() >= 64 { break; }
                }
            }
        }
        self.lensing_pipeline.execute(
            &mut encoder,
            &self.queue,
            &lens_settings,
            &lens_data,
            &self.lensing_textures.lensed_view,
        );

        // ── Bloom post-processing ──────────────────────────────────
        if direct_to_swapchain {
            self.bloom_pipeline.render_passthrough(&mut encoder, &frame_view);
        } else {
            let (bloom_threshold, bloom_intensity) = world
                .get_resource::<BloomSettings>()
                .map(|s| if s.enabled { (s.threshold, s.intensity) } else { (100.0, 0.0) })
                .unwrap_or((1.0, 0.0));
            self.bloom_pipeline.update_uniforms(&self.queue, bloom_threshold, bloom_intensity);
            self.bloom_pipeline.execute(
                &mut encoder,
                &self.bloom_textures.bloom_view,
                &self.bloom_textures.ping_view,
                &frame_view,
            );
        }

        // ── Pass 2: Lines overlay — drawn AFTER tonemapping directly
        //    onto swapchain so they bypass HDR/bloom and stay visible.
        //    Uses the sphere depth buffer (loaded, not cleared) for
        //    proper occlusion behind solid geometry.
        if !line_vertices.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("line_overlay_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.sphere_pipeline.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.line_pipeline.render(&mut pass, line_vertices.len() as u32);
        }

        // ── Egui UI pass — separate encoder to avoid lifetime issues ──
        let mut ui_actions = Vec::new();
        let mut egui_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("egui_encoder"),
            });
        {
            let egui_input = self.egui_state.take_egui_input(&*self.window);
            
            // Read energy monitor for UI
            let energy_monitor = world.get_resource::<crate::physics::energy::EnergyMonitor>();
            let diagnostics = world.get_resource::<crate::core::diagnostics::Diagnostics>();
            
            let egui_output = self.egui_ctx.run_ui(egui_input, |ctx| {
                ui_actions = super::ui::draw_ui(
                    ctx,
                    &mut self.ui_state,
                    &entity_list,
                    current_selected,
                    hud_data.as_ref(),
                    energy_monitor,
                    diagnostics,
                );
            });
            let pp = egui_output.pixels_per_point;
            self.egui_state.handle_platform_output(&*self.window, egui_output.platform_output);

            for action in ui_actions {
                match action {
                    super::ui::actions::UiAction::SelectEntity(e) => {
                        if let Some(mut sel) = world.get_resource_mut::<crate::render::SelectedEntity>() {
                            sel.0 = Some(e);
                        }
                    }
                    super::ui::actions::UiAction::ClearSelection => {
                        if let Some(mut sel) = world.get_resource_mut::<crate::render::SelectedEntity>() {
                            sel.0 = None;
                        }
                    }
                    super::ui::actions::UiAction::FocusCamera(e) => {
                        if let (Some(local), Some(sector)) = (
                            world.get::<crate::core::coordinates::LocalPosition>(e),
                            world.get::<crate::core::coordinates::Sector>(e),
                        ) {
                            let config = world.get_resource::<crate::core::config::UniverseConfig>().unwrap();
                            let ss = config.sector_size;
                            let world_pos = glam::DVec3::new(
                                sector.0.x as f64 * ss,
                                sector.0.y as f64 * ss,
                                sector.0.z as f64 * ss,
                            ) + local.0;

                            let target_radius = world.get::<crate::components::spatial::BoundingRadius>(e).map(|r| r.0).unwrap_or(10.0);
                            let scale_mode = world.get_resource::<crate::render::RenderScaleMode>().copied().unwrap_or(crate::render::RenderScaleMode::Linear);
                            let visual_radius = match scale_mode {
                                crate::render::RenderScaleMode::Linear => target_radius,
                                crate::render::RenderScaleMode::Logarithmic => {
                                    let mass = world.get::<crate::components::Mass>(e).map(|m| m.0).unwrap_or(target_radius * target_radius * target_radius * 1000.0);
                                    let r_base = 5.0_f64;
                                    let k = 15.0_f64;
                                    let m_ref = 1e22_f64;
                                    r_base + k * (1.0 + mass / m_ref).log10()
                                }
                            };
                            let target_distance = visual_radius * 3.0;

                            if let Some(mut camera_mode) = world.get_resource_mut::<crate::render::CameraMode>() {
                                match &mut *camera_mode {
                                    crate::render::CameraMode::FreeFly(_) => {
                                        *camera_mode = crate::render::CameraMode::Orbital(crate::render::OrbitalCamera {
                                            target: world_pos,
                                            distance: target_distance,
                                            ..Default::default()
                                        });
                                    }
                                    crate::render::CameraMode::Orbital(o) => {
                                        o.target = world_pos;
                                        o.distance = target_distance;
                                    }
                                }
                            }
                        }
                    }
                    super::ui::actions::UiAction::TogglePause => {
                        if let Some(mut t) = world.get_resource_mut::<crate::core::SimulationTime>() {
                            t.paused = !t.paused;
                            self.ui_state.console_history.push(format!("Simulation {}", if t.paused { "paused" } else { "resumed" }));
                        }
                    }
                    super::ui::actions::UiAction::StepTick => {
                        if let Some(mut t) = world.get_resource_mut::<crate::core::SimulationTime>() {
                            t.paused = true;
                            // Step exactly one tick. We can do this by setting accumulator.
                            t.accumulator += t.dt;
                        }
                    }
                    super::ui::actions::UiAction::SetTimeScale(scale) => {
                        if let Some(mut t) = world.get_resource_mut::<crate::core::SimulationTime>() {
                            t.speed_multiplier = scale;
                        }
                    }
                    super::ui::actions::UiAction::QuickSave => {
                        let mut saved_sim = crate::core::serialization::SavedSimulation { entities: Vec::new() };
                        let mut query = world.query::<(
                            Option<&crate::components::identifiers::EntityName>,
                            Option<&crate::components::identifiers::BodyType>,
                            Option<&crate::components::Mass>,
                            Option<&crate::components::spatial::BoundingRadius>,
                            Option<&crate::core::coordinates::LocalPosition>,
                            Option<&crate::core::coordinates::Sector>,
                            Option<&crate::components::Velocity>,
                            Option<&crate::components::Temperature>,
                        )>();
                        for (name, body_type, mass, radius, pos, sector, vel, temp) in query.iter(world) {
                            saved_sim.entities.push(crate::core::serialization::SavedEntity {
                                name: name.cloned(),
                                body_type: body_type.cloned(),
                                mass: mass.cloned(),
                                radius: radius.cloned(),
                                local_pos: pos.cloned(),
                                sector: sector.cloned(),
                                velocity: vel.cloned(),
                                temperature: temp.cloned(),
                            });
                        }
                        match std::fs::File::create("quicksave.bin") {
                            Ok(file) => {
                                if let Err(e) = bincode::serialize_into(file, &saved_sim) {
                                    self.ui_state.console_history.push(format!("QuickSave failed: {}", e));
                                } else {
                                    self.ui_state.console_history.push("QuickSave successful".into());
                                }
                            }
                            Err(e) => {
                                self.ui_state.console_history.push(format!("Failed to create save file: {}", e));
                            }
                        }
                    }
                    super::ui::actions::UiAction::QuickLoad => {
                        match std::fs::File::open("quicksave.bin") {
                            Ok(file) => {
                                match bincode::deserialize_from::<_, crate::core::serialization::SavedSimulation>(file) {
                                    Ok(saved_sim) => {
                                        // Despawn all current entities with LocalPosition
                                        let mut entities_to_despawn = Vec::new();
                                        let mut query = world.query_filtered::<Entity, bevy_ecs::query::With<crate::core::coordinates::LocalPosition>>();
                                        for entity in query.iter(world) {
                                            entities_to_despawn.push(entity);
                                        }
                                        for entity in entities_to_despawn {
                                            world.despawn(entity);
                                        }
                                        // Spawn new entities
                                        for saved_entity in saved_sim.entities {
                                            let mut e = world.spawn_empty();
                                            if let Some(c) = saved_entity.name { e.insert(c); }
                                            if let Some(c) = saved_entity.body_type { e.insert(c); }
                                            if let Some(c) = saved_entity.mass { e.insert(c); }
                                            if let Some(c) = saved_entity.radius { e.insert(c); }
                                            if let Some(c) = saved_entity.local_pos { e.insert(c); }
                                            if let Some(c) = saved_entity.sector { e.insert(c); }
                                            if let Some(c) = saved_entity.velocity { e.insert(c); }
                                            if let Some(c) = saved_entity.temperature { e.insert(c); }
                                            e.insert(crate::components::Acceleration(glam::DVec3::ZERO));
                                            e.insert(crate::components::Force(glam::DVec3::ZERO));
                                            let mass_val = saved_entity.mass.map(|m| m.0).unwrap_or(1.0);
                                            let vel_val = saved_entity.velocity.map(|v| v.0).unwrap_or(glam::DVec3::ZERO);
                                            e.insert(crate::components::LinearMomentum(vel_val * mass_val));
                                        }
                                        self.ui_state.console_history.push("QuickLoad successful".into());
                                    }
                                    Err(e) => {
                                        self.ui_state.console_history.push(format!("QuickLoad failed: {}", e));
                                    }
                                }
                            }
                            Err(e) => {
                                self.ui_state.console_history.push(format!("Failed to open save file: {}", e));
                            }
                        }
                    }
                    super::ui::actions::UiAction::SpawnEntity { mass, position, velocity, temperature, body_type } => {
                        self.ui_state.console_history.push(format!("SpawnEntity: mass={mass:.1e}, type={body_type:?}"));
                        let mut e = world.spawn_empty();
                        e.insert(crate::components::identifiers::EntityName(format!("Spawned {:?}", body_type)));
                        e.insert(body_type);
                        e.insert(crate::components::Mass(mass));
                        e.insert(crate::components::Velocity(velocity));
                        e.insert(crate::components::Temperature(temperature));
                        e.insert(crate::core::coordinates::Sector(glam::I64Vec3::ZERO));
                        e.insert(crate::core::coordinates::LocalPosition(position));
                        let default_density = match body_type {
                            crate::components::BodyType::BlackHole => 1e16,
                            crate::components::BodyType::Star => 1.4e3,
                            crate::components::BodyType::Asteroid => 3000.0,
                            _ => 5500.0,
                        };
                        let radius = ((mass / default_density) / (4.0/3.0 * std::f64::consts::PI)).cbrt();
                        e.insert(crate::components::spatial::BoundingRadius(radius));
                        e.insert(crate::components::Force(glam::DVec3::ZERO));
                        e.insert(crate::components::Acceleration(glam::DVec3::ZERO));
                        e.insert(crate::components::LinearMomentum(velocity * mass));
                    }
                    super::ui::actions::UiAction::ExecuteCommand(cmd) => {
                        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
                        match parts.get(0).copied() {
                            Some("clear") => {
                                self.ui_state.console_history.clear();
                            }
                            Some("help") => {
                                self.ui_state.console_history.push("Commands: clear, help".into());
                            }
                            Some(_) => {
                                self.ui_state.console_history.push(format!("Command '{cmd}' not recognized"));
                            }
                            None => {}
                        }
                    }
                }
            }

            for (id, image_delta) in &egui_output.textures_delta.set {
                self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
            }

            let clipped_meshes = self.egui_ctx.tessellate(egui_output.shapes, pp);

            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: pp,
            };

            // egui_wgpu requires RenderPass<'static>. The encoder lives on the
            // stack for the full pass — transmute is sound here.
            self.egui_renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut egui_encoder,
                &clipped_meshes,
                &screen_descriptor,
            );

            let egui_encoder_static: &'static mut wgpu::CommandEncoder =
                unsafe { std::mem::transmute(&mut egui_encoder) };
            let mut pass = egui_encoder_static.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.egui_renderer.render(
                &mut pass,
                &clipped_meshes,
                &screen_descriptor,
            );
            drop(pass);

            for id in &egui_output.textures_delta.free {
                self.egui_renderer.free_texture(id);
            }
        }

        // ── Submit encoders and present ─────────────────────────────
        self.queue.submit([
            encoder.finish(),
            egui_encoder.finish(),
        ]);
        frame.present();
    }
}

/// Approximate blackbody color from temperature (Wien's law approximation).
///
/// Returns HDR values (>1.0) for very hot stars so bloom can extract them.
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
    let mut color = [r / 255.0, g / 255.0, b / 255.0, 1.0];
    // Boost hot stars (T > 8000K) into HDR range for bloom
    if temp_kelvin > 8000.0 {
        let boost = 1.0 + (temp_kelvin - 8000.0) / 4000.0; // 1× at 8kK, 6× at 28kK
        color[0] *= boost;
        color[1] *= boost;
        color[2] *= boost;
    }
    [color[0] as f32, color[1] as f32, color[2] as f32, color[3] as f32]
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
