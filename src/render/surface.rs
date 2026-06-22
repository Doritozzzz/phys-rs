//! wgpu render surface, device, queue, pipeline, and frame presentation.

use std::f64::consts::FRAC_PI_4;
use bevy_ecs::prelude::*;
use glam::{DMat4, DVec3};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use super::render_pipeline::{InstanceData, SpherePipeline};
use crate::components::identifiers::BodyType;
use crate::components::material::Temperature;
use crate::components::spatial::BoundingRadius;
use crate::core::coordinates::{LocalPosition, Sector};
use crate::core::config::UniverseConfig;
use super::camera::CameraState;

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

        let sphere_pipeline = SpherePipeline::new(&device, &config, &queue);

        Self {
            surface,
            device,
            queue,
            config,
            sphere_pipeline,
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
            wgpu::CurrentSurfaceTexture::Outdated => {
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                panic!("wgpu surface lost");
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // ── Build view-projection matrix from CameraState ──────────
        let (view_proj, ss) = {
            let cam = world.get_resource::<CameraState>();
            let config = world.get_resource::<UniverseConfig>();
            match (cam, config) {
                (Some(cam), Some(config)) => {
                    let aspect = self.config.width as f64 / self.config.height as f64;
                    let proj = DMat4::perspective_rh(FRAC_PI_4 * 0.5, aspect, 0.1, 1.0e15);
                    let vw = DMat4::look_at_rh(cam.position, cam.position + cam.forward, cam.up);
                    (proj * vw, config.sector_size)
                }
                _ => (DMat4::IDENTITY, 1e12),
            }
        };

        let view_proj_f32: glam::Mat4 = glam::Mat4::from_cols(
            view_proj.x_axis.as_vec4().into(),
            view_proj.y_axis.as_vec4().into(),
            view_proj.z_axis.as_vec4().into(),
            view_proj.w_axis.as_vec4().into(),
        );

        // ── Collect instance data from ECS ─────────────────────────
        let mut instances: Vec<InstanceData> = Vec::new();

        let cam_pos = world.get_resource::<CameraState>()
            .map(|c| c.position)
            .unwrap_or(DVec3::ZERO);

        let mut entity_query = world.query::<(
            &Sector, &LocalPosition, &BoundingRadius, &BodyType,
            Option<&Temperature>,
        )>();
        for (sector, local, radius, body_type, temp) in entity_query.iter(world) {
            let world_pos = DVec3::new(
                sector.0.x as f64 * ss,
                sector.0.y as f64 * ss,
                sector.0.z as f64 * ss,
            ) + local.0;

            let relative = world_pos - cam_pos;

            let color = temp
                .map(|t| temperature_to_rgb(t.0))
                .unwrap_or(match body_type {
                    BodyType::BlackHole => [0.0, 0.0, 0.0, 1.0],
                    BodyType::Star => [0.5, 0.5, 0.2, 1.0],
                    BodyType::FluidParticle => [0.2, 0.5, 0.8, 0.6],
                    _ => [0.4, 0.4, 0.5, 1.0],
                });

            instances.push(InstanceData {
                position: [relative.x as f32, relative.y as f32, relative.z as f32],
                scale: radius.0 as f32,
                color,
            });
        }

        // ── Upload uniforms and instances ─────────────────────────
        self.sphere_pipeline.update_uniform(&self.queue, &view_proj_f32);
        self.sphere_pipeline.update_instances(&self.queue, &instances);

        // ── Encode and submit render pass ─────────────────────────
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        self.sphere_pipeline.render(
            &mut encoder,
            &view,
            instances.len() as u32,
        );

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
