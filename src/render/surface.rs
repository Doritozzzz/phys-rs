//! wgpu render surface, device, queue, and frame presentation.

use winit::dpi::PhysicalSize;
use winit::window::Window;

/// Owns the wgpu device, queue, surface, and configuration for rendering.
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

        Self {
            surface,
            device,
            queue,
            config,
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
    }

    /// Acquire the next frame texture, clear to dark navy, and present.
    pub fn present_frame(&self) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return,
            wgpu::CurrentSurfaceTexture::Outdated => {
                // Surface configuration is stale — caller should resize.
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                panic!("wgpu surface lost");
            }
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                depth_stencil_attachment: None,
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // Render draws added in Phase S2.
            drop(pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}
