//! Sandbox: interactive visualization layer.
//!
//! Owns the winit event loop, wgpu render surface, camera, and input handling.
//! Consumes the engine (World + Schedule) but never modifies physics code.

pub mod camera;
pub mod render_pipeline;
pub mod surface;

use std::sync::Arc;
use std::time::Instant;
use bevy_ecs::prelude::*;
use camera::{CameraInput, CameraMode, CameraState, FreeFlyCamera, OrbitalCamera};
use glam::DVec3;
use surface::RenderContext;
use crate::components::spatial::{BoundingRadius, OrbitTrail};
use crate::core::coordinates::{LocalPosition, Sector};
use crate::core::config::UniverseConfig;
use crate::core::SimulationTime;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

// ── Rendering resources ─────────────────────────────────────────────────

/// Window pixel dimensions. Updated on resize.
#[derive(Resource)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl Default for WindowSize {
    fn default() -> Self { Self { width: 1280, height: 720 } }
}

/// Pending raycast from a left-click. Cleared by selection_system.
#[derive(Resource)]
pub struct RaycastClick(pub Option<(f64, f64)>);

impl Default for RaycastClick {
    fn default() -> Self { Self(None) }
}

/// Currently selected physics entity.
#[derive(Resource)]
pub struct SelectedEntity(pub Option<Entity>);

impl Default for SelectedEntity {
    fn default() -> Self { Self(None) }
}

// ── Frame timing ─────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct FrameTiming {
    pub frame_count: u64,
    pub fps_accum: f64,
    pub display_fps: f64,
    pub display_tps: f64,
}

impl Default for FrameTiming {
    fn default() -> Self {
        Self { frame_count: 0, fps_accum: 0.0, display_fps: 0.0, display_tps: 0.0 }
    }
}

// ── S2.4: Visual scale mode ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub enum RenderScaleMode {
    /// Physical linear scale (real BoundingRadius).
    Linear,
    /// Logarithmic scale based on mass for visual clarity.
    Logarithmic,
}

impl Default for RenderScaleMode {
    fn default() -> Self { Self::Linear }
}

// ── S2.6 / S2.7: Debug vector toggles ───────────────────────────────────

#[derive(Resource, Clone, Copy)]
pub struct ShowVelocityVectors(pub bool);

impl Default for ShowVelocityVectors {
    fn default() -> Self { Self(false) }
}

#[derive(Resource, Clone, Copy)]
pub struct ShowForceVectors(pub bool);

impl Default for ShowForceVectors {
    fn default() -> Self { Self(false) }
}

// ── S2.8: Sector grid toggle ────────────────────────────────────────────

#[derive(Resource, Clone, Copy)]
pub struct ShowSectorGrid(pub bool);

impl Default for ShowSectorGrid {
    fn default() -> Self { Self(false) }
}

// ── App ───────────────────────────────────────────────────────────────────

/// Top-level sandbox application.
pub struct App {
    pub world: World,
    pub schedule: Schedule,
    pub render_schedule: Schedule,
    pub render_state: Option<RenderContext>,
    pub window: Option<Arc<Window>>,
    pub last_frame_time: Option<Instant>,
}

impl App {
    pub fn new(mut world: World, schedule: Schedule) -> Self {
        world.init_resource::<CameraMode>();
        world.init_resource::<CameraInput>();
        world.init_resource::<CameraState>();
        {
            let mut cs = world.get_resource_mut::<CameraState>().unwrap();
            let yaw = 0.0_f64;
            let pitch = -0.3_f64;
            let forward = DVec3::new(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos()).normalize();
            let right = forward.cross(DVec3::Y).normalize();
            *cs = CameraState {
                position: DVec3::new(0.0, 500.0, 2000.0),
                forward,
                right,
                up: right.cross(forward),
            };
        }
        world.init_resource::<WindowSize>();
        world.init_resource::<SelectedEntity>();
        world.init_resource::<RaycastClick>();
        world.init_resource::<FrameTiming>();
        world.init_resource::<RenderScaleMode>();
        world.init_resource::<ShowVelocityVectors>();
        world.init_resource::<ShowForceVectors>();
        world.init_resource::<ShowSectorGrid>();

        let mut render_schedule = Schedule::default();
        render_schedule.add_systems(camera::camera_system);
        render_schedule.add_systems(selection_system);
        render_schedule.add_systems(orbit_trail_system);

        Self {
            world,
            schedule,
            render_schedule,
            render_state: None,
            window: None,
            last_frame_time: None,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.render_state.take();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("phys-rs")
                        .with_inner_size(PhysicalSize::new(1280, 720)),
                )
                .expect("Failed to create window"),
        );
        let render_state = RenderContext::new(&window);
        self.window = Some(window);
        self.render_state = Some(render_state);
        self.last_frame_time = Some(Instant::now());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(rs) = &mut self.render_state {
                    rs.resize(size);
                }
                if let Some(mut ws) = self.world.get_resource_mut::<WindowSize>() {
                    ws.width = size.width;
                    ws.height = size.height;
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if let PhysicalKey::Code(code) = event.physical_key {
                    match code {
                        // Camera movement
                        KeyCode::KeyW | KeyCode::KeyS
                        | KeyCode::KeyA | KeyCode::KeyD
                        | KeyCode::KeyE | KeyCode::KeyQ => {
                            if let Some(mut input) = self.world.get_resource_mut::<CameraInput>() {
                                match code {
                                    KeyCode::KeyW => input.forward = pressed,
                                    KeyCode::KeyS => input.backward = pressed,
                                    KeyCode::KeyA => input.strafe_left = pressed,
                                    KeyCode::KeyD => input.strafe_right = pressed,
                                    KeyCode::KeyE => input.ascend = pressed,
                                    KeyCode::KeyQ => input.descend = pressed,
                                    _ => {}
                                }
                            }
                        }
                        // Speed multiplier
                        KeyCode::Digit1 if pressed => {
                            if let Some(mut t) = self.world.get_resource_mut::<SimulationTime>() {
                                t.speed_multiplier = 1.0;
                            }
                        }
                        KeyCode::Digit2 if pressed => {
                            if let Some(mut t) = self.world.get_resource_mut::<SimulationTime>() {
                                t.speed_multiplier = 10.0;
                            }
                        }
                        KeyCode::Digit3 if pressed => {
                            if let Some(mut t) = self.world.get_resource_mut::<SimulationTime>() {
                                t.speed_multiplier = 100.0;
                            }
                        }
                        KeyCode::Digit4 if pressed => {
                            if let Some(mut t) = self.world.get_resource_mut::<SimulationTime>() {
                                t.speed_multiplier = 1000.0;
                            }
                        }
                        // Pause/resume
                        KeyCode::Space if pressed => {
                            if let Some(mut t) = self.world.get_resource_mut::<SimulationTime>() {
                                t.paused = !t.paused;
                            }
                        }
                        // Reset simulation time
                        KeyCode::KeyR if pressed => {
                            if let Some(mut t) = self.world.get_resource_mut::<SimulationTime>() {
                                t.tick = 0;
                                t.elapsed = 0.0;
                                t.accumulator = 0.0;
                            }
                        }
                        // Toggle orbital / free-fly camera
                        KeyCode::KeyO if pressed => {
                            let mut mode = self.world.get_resource_mut::<CameraMode>().unwrap();
                            match &*mode {
                                CameraMode::FreeFly(ff) => {
                                    *mode = CameraMode::Orbital(OrbitalCamera {
                                        target: ff.position.clone(),
                                        ..Default::default()
                                    });
                                }
                                CameraMode::Orbital(_) => {
                                    *mode = CameraMode::FreeFly(FreeFlyCamera::default());
                                }
                            }
                        }
                        // ── S2.4: Toggle linear / logarithmic scale ─────
                        KeyCode::KeyL if pressed => {
                            let mut mode = self.world.get_resource_mut::<RenderScaleMode>().unwrap();
                            *mode = match *mode {
                                RenderScaleMode::Linear => RenderScaleMode::Logarithmic,
                                RenderScaleMode::Logarithmic => RenderScaleMode::Linear,
                            };
                            println!("→ Render scale: {:?}", *mode);
                        }
                        // ── S2.6: Toggle velocity vectors ──────────────
                        KeyCode::KeyV if pressed => {
                            let mut show = self.world.get_resource_mut::<ShowVelocityVectors>().unwrap();
                            show.0 = !show.0;
                            println!("→ Velocity vectors: {}", show.0);
                        }
                        // ── S2.7: Toggle force vectors ──────────────────
                        KeyCode::KeyF if pressed => {
                            let mut show = self.world.get_resource_mut::<ShowForceVectors>().unwrap();
                            show.0 = !show.0;
                            println!("→ Force vectors: {}", show.0);
                        }
                        // ── S2.8: Toggle sector grid ────────────────────
                        KeyCode::KeyG if pressed => {
                            let mut show = self.world.get_resource_mut::<ShowSectorGrid>().unwrap();
                            show.0 = !show.0;
                            println!("→ Sector grid: {}", show.0);
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                if let Some(mut input) = self.world.get_resource_mut::<CameraInput>() {
                    match button {
                        winit::event::MouseButton::Left => input.left_mouse = pressed,
                        winit::event::MouseButton::Right => input.right_mouse = pressed,
                        _ => {}
                    }
                }
                if pressed && button == winit::event::MouseButton::Right {
                    self.world.resource_scope::<CameraMode, _>(|_world, mut mode| {
                        if let CameraMode::Orbital(o) = &*mode {
                            let eye = o.target
                                + DVec3::new(
                                    o.distance * o.phi.cos() * o.theta.sin(),
                                    o.distance * o.phi.sin(),
                                    o.distance * o.phi.cos() * o.theta.cos(),
                                );
                            let look = (o.target - eye).normalize();
                            let mut ff = camera::FreeFlyCamera::default();
                            ff.position = eye;
                            ff.yaw = look.z.atan2(look.x);
                            ff.pitch = look.y.asin();
                            *mode = CameraMode::FreeFly(ff);
                        }
                    });
                }
                if pressed && button == winit::event::MouseButton::Left {
                    let pos = self.world.get_resource::<CameraInput>()
                        .map(|c| (c.cursor_x, c.cursor_y));
                    if let Some((cx, cy)) = pos {
                        if let Some(mut rc) = self.world.get_resource_mut::<RaycastClick>() {
                            rc.0 = Some((cx, cy));
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(mut input) = self.world.get_resource_mut::<CameraInput>() {
                    if input.cursor_initialized {
                        input.mouse_delta_x += position.x - input.cursor_x;
                        input.mouse_delta_y += position.y - input.cursor_y;
                    }
                    input.cursor_x = position.x;
                    input.cursor_y = position.y;
                    input.cursor_initialized = true;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(mut input) = self.world.get_resource_mut::<CameraInput>() {
                    let y = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y as f64,
                        MouseScrollDelta::PixelDelta(pos) => pos.y,
                    };
                    input.scroll_delta += y;
                }
            }
            WindowEvent::RedrawRequested => {
                // Reset frame-accumulated input
                if let Some(mut input) = self.world.get_resource_mut::<CameraInput>() {
                    input.mouse_delta_x = 0.0;
                    input.mouse_delta_y = 0.0;
                    input.scroll_delta = 0.0;
                }

                // Fixed-timestep accumulator
                let now = Instant::now();
                let frame_dt = self
                    .last_frame_time
                    .map(|t| (now - t).as_secs_f64().min(0.1))
                    .unwrap_or(1.0 / 60.0);
                self.last_frame_time = Some(now);

                let ticks = {
                    let mut time = self.world.get_resource_mut::<SimulationTime>().unwrap();
                    time.accumulate(frame_dt)
                };

                for _ in 0..ticks {
                    self.schedule.run(&mut self.world);
                    if let Some(mut time) = self.world.get_resource_mut::<SimulationTime>() {
                        time.advance_tick();
                    }
                }

                // Update camera + selection + trails
                self.render_schedule.run(&mut self.world);

                // Frame timing
                let ec = self.world.entities().len();
                if let Some(mut ft) = self.world.get_resource_mut::<FrameTiming>() {
                    ft.frame_count += 1;
                    ft.fps_accum += 1.0 / frame_dt.max(1e-10);
                    if ft.frame_count % 60 == 0 {
                        ft.display_fps = ft.fps_accum / 60.0;
                        ft.fps_accum = 0.0;
                        ft.display_tps = ticks as f64 / frame_dt;
                        println!("FPS: {:.1} | TPS: {:.1} | Entities: {}",
                            ft.display_fps, ft.display_tps, ec);
                    }
                }

                // Present frame
                if let Some(rs) = &self.render_state {
                    rs.present_frame(&mut self.world);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

// ── Selection system ─────────────────────────────────────────────────────

fn selection_system(
    camera_state: Res<CameraState>,
    mut raycast: ResMut<RaycastClick>,
    window_size: Res<WindowSize>,
    config: Res<UniverseConfig>,
    mut selected: ResMut<SelectedEntity>,
    mut camera_mode: ResMut<CameraMode>,
    entities: Query<(Entity, &Sector, &LocalPosition, &BoundingRadius)>,
) {
    let Some((cx, cy)) = raycast.0.take() else { return };

    let w = window_size.width as f64;
    let h = window_size.height as f64;
    if w == 0.0 || h == 0.0 { return; }

    let aspect = w / h;
    let fov_half = (std::f64::consts::FRAC_PI_4 * 0.5).tan();
    let ndc_x = (2.0 * cx / w - 1.0) * fov_half * aspect;
    let ndc_y = (1.0 - 2.0 * cy / h) * fov_half;

    let ro = camera_state.position;
    let rd = (camera_state.forward + camera_state.right * ndc_x + camera_state.up * ndc_y).normalize();

    let ss = config.sector_size;
    let mut hit: Option<(Entity, DVec3)> = None;
    let mut closest_t = f64::MAX;

    for (entity, sector, local, radius) in &entities {
        let center = DVec3::new(
            sector.0.x as f64 * ss,
            sector.0.y as f64 * ss,
            sector.0.z as f64 * ss,
        ) + local.0;

        let oc = center - ro;
        let a = rd.dot(rd);
        let b = 2.0 * oc.dot(rd);
        let c = oc.dot(oc) - radius.0 * radius.0;
        let disc = b * b - 4.0 * a * c;

        if disc >= 0.0 {
            let t = (-b - disc.sqrt()) / (2.0 * a);
            if t > 0.0 && t < closest_t {
                closest_t = t;
                hit = Some((entity, center));
            }
        }
    }

    if let Some((entity, world_pos)) = hit {
        selected.0 = Some(entity);
        println!("→ Selected entity {} at [{:.3e}, {:.3e}, {:.3e}]",
            entity.index(), world_pos.x, world_pos.y, world_pos.z);

        match &mut *camera_mode {
            CameraMode::FreeFly(_) => {
                *camera_mode = CameraMode::Orbital(OrbitalCamera {
                    target: world_pos,
                    ..Default::default()
                });
            }
            CameraMode::Orbital(o) => {
                o.target = world_pos;
            }
        }
    } else {
        selected.0 = None;
    }
}

// ── S2.5: Orbit trail collection system ─────────────────────────────────

/// Updates OrbitTrail components: pushes current position every N physics ticks.
fn orbit_trail_system(
    mut trails: Query<(&mut OrbitTrail, &Sector, &LocalPosition)>,
    time: Res<SimulationTime>,
) {
    // Update trail every 4 physics ticks
    if time.tick % 4 != 0 { return; }

    for (mut trail, sector, local) in &mut trails {
        trail.history.push_front((*sector, *local));
        while trail.history.len() > trail.max_points {
            trail.history.pop_back();
        }
    }
}
