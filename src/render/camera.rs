//! Camera resources and systems.
//!
//! Free-fly (WASD + mouse-look) and orbital (click-drag around target) modes.
//! Both expose a view-projection matrix for the rendering pipeline (S2).

use bevy_ecs::prelude::*;
use glam::DVec3;

// ── Free-fly camera ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct FreeFlyCamera {
    pub position: DVec3,
    pub yaw: f64,
    pub pitch: f64,
    pub speed: f64,
    pub sensitivity: f64,
}

impl Default for FreeFlyCamera {
    fn default() -> Self {
        Self {
            position: DVec3::new(0.0, 80.0, 300.0),
            yaw: 0.0,
            pitch: -0.3,
            speed: 100.0,
            sensitivity: 0.005,
        }
    }
}

// ── Orbital camera ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct OrbitalCamera {
    pub target: DVec3,
    pub distance: f64,
    pub theta: f64,
    pub phi: f64,
    pub sensitivity: f64,
    pub pan_sensitivity: f64,
    pub min_distance: f64,
    pub max_distance: f64,
}

impl Default for OrbitalCamera {
    fn default() -> Self {
        Self {
            target: DVec3::ZERO,
            distance: 300.0,
            theta: 0.0,
            phi: 1.4,
            sensitivity: 0.008,
            pan_sensitivity: 0.002,
            min_distance: 0.1,
            max_distance: 1.0e12,
        }
    }
}

// ── Camera mode ───────────────────────────────────────────────────────────

#[derive(Resource)]
pub enum CameraMode {
    FreeFly(FreeFlyCamera),
    Orbital(OrbitalCamera),
}

impl Default for CameraMode {
    fn default() -> Self {
        CameraMode::FreeFly(FreeFlyCamera::default())
    }
}

// ── Input state (per frame) ───────────────────────────────────────────────

#[derive(Resource, Clone)]
pub struct CameraInput {
    pub forward: bool,
    pub backward: bool,
    pub strafe_left: bool,
    pub strafe_right: bool,
    pub ascend: bool,
    pub descend: bool,
    pub mouse_delta_x: f64,
    pub mouse_delta_y: f64,
    pub scroll_delta: f64,
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub cursor_initialized: bool,
    pub left_mouse: bool,
    pub right_mouse: bool,
}

impl Default for CameraInput {
    fn default() -> Self {
        Self {
            forward: false,
            backward: false,
            strafe_left: false,
            strafe_right: false,
            ascend: false,
            descend: false,
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
            scroll_delta: 0.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_initialized: false,
            left_mouse: false,
            right_mouse: false,
        }
    }
}

/// Current camera position and orientation.
/// Updated by camera_system, consumed by selection and render pipeline.
#[derive(Resource)]
pub struct CameraState {
    pub position: DVec3,
    pub forward: DVec3,
    pub right: DVec3,
    pub up: DVec3,
}

impl Default for CameraState {
    fn default() -> Self {
        Self { position: DVec3::ZERO, forward: DVec3::NEG_Z, right: DVec3::X, up: DVec3::Y }
    }
}

// ── Camera system ─────────────────────────────────────────────────────────

pub fn camera_system(
    time: Res<crate::core::SimulationTime>,
    input: Res<CameraInput>,
    mut mode: ResMut<CameraMode>,
    mut state: ResMut<CameraState>,
) {
    match &mut *mode {
        CameraMode::FreeFly(cam) => update_free_fly(cam, &time, &input, &mut state),
        CameraMode::Orbital(cam) => update_orbital(cam, &input, &mut state),
    }
}

pub(crate) fn update_free_fly(cam: &mut FreeFlyCamera, time: &crate::core::SimulationTime, input: &CameraInput, state: &mut CameraState) {
    let dt = time.dt;

    cam.yaw -= input.mouse_delta_x * cam.sensitivity;
    cam.pitch += input.mouse_delta_y * cam.sensitivity;
    cam.pitch = cam.pitch.clamp(-1.5, 1.5);

    if input.scroll_delta != 0.0 {
        let factor = 2.0_f64.powf(input.scroll_delta * 0.25);
        cam.speed = (cam.speed * factor).clamp(1.0, 1.0e12);
    }

    let forward_h = DVec3::new(cam.yaw.cos(), 0.0, cam.yaw.sin()).normalize();
    let right = forward_h.cross(DVec3::Y).normalize();

    let mut movement = DVec3::ZERO;
    if input.forward { movement += forward_h; }
    if input.backward { movement -= forward_h; }
    if input.strafe_right { movement += right; }
    if input.strafe_left { movement -= right; }
    if input.ascend { movement += DVec3::Y; }
    if input.descend { movement -= DVec3::Y; }

    if movement.length_squared() > 0.0 {
        cam.position += movement.normalize() * cam.speed * dt;
    }

    state.position = cam.position;
    state.forward = DVec3::new(cam.yaw.cos() * cam.pitch.cos(), cam.pitch.sin(), cam.yaw.sin() * cam.pitch.cos()).normalize();
    state.right = state.forward.cross(DVec3::Y).normalize();
    state.up = state.right.cross(state.forward);
}

pub(crate) fn update_orbital(cam: &mut OrbitalCamera, input: &CameraInput, state: &mut CameraState) {
    if input.left_mouse {
        cam.theta -= input.mouse_delta_x * cam.sensitivity;
        cam.phi += input.mouse_delta_y * cam.sensitivity;
        cam.phi = cam.phi.clamp(0.01, std::f64::consts::PI - 0.01);
    }

    if input.scroll_delta != 0.0 {
        let factor = 1.0 / 1.1_f64.powf(input.scroll_delta);
        cam.distance = (cam.distance * factor).clamp(cam.min_distance, cam.max_distance);
    }

    let eye = cam.target + DVec3::new(
        cam.distance * cam.phi.cos() * cam.theta.sin(),
        cam.distance * cam.phi.sin(),
        cam.distance * cam.phi.cos() * cam.theta.cos(),
    );

    // Pan (right mouse) — use current view vectors before target changes
    if input.right_mouse {
        let forward = (cam.target - eye).normalize();
        let right = forward.cross(DVec3::Y).normalize();
        let up = right.cross(forward);
        let speed = cam.distance * cam.pan_sensitivity;
        cam.target -= right * input.mouse_delta_x * speed;
        cam.target += up * input.mouse_delta_y * speed;
    }

    // Recompute eye with final target
    let eye = cam.target + DVec3::new(
        cam.distance * cam.phi.cos() * cam.theta.sin(),
        cam.distance * cam.phi.sin(),
        cam.distance * cam.phi.cos() * cam.theta.cos(),
    );
    state.position = eye;
    state.forward = (cam.target - eye).normalize();
    state.right = state.forward.cross(DVec3::Y).normalize();
    state.up = state.right.cross(state.forward);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::SimulationTime;

    fn default_sim_time() -> SimulationTime {
        SimulationTime::with_dt(1.0 / 60.0)
    }

    fn zero_input() -> CameraInput {
        CameraInput::default()
    }

    // ── Free-fly tests ──────────────────────────────────────────

    #[test]
    fn test_free_fly_forward_length() {
        let cam = FreeFlyCamera::default();
        let forward = DVec3::new(
            cam.yaw.cos() * cam.pitch.cos(),
            cam.pitch.sin(),
            cam.yaw.sin() * cam.pitch.cos(),
        )
        .normalize();
        assert!(forward.is_finite());
        assert!((forward.length() - 1.0).abs() < 1e-12, "forward must be unit length");
    }

    #[test]
    fn test_free_fly_identity_with_zero_dt() {
        let mut cam = FreeFlyCamera::default();
        let pos_before = cam.position;
        let time = default_sim_time();
        let input = zero_input();
        let mut state = CameraState::default();

        update_free_fly(&mut cam, &time, &input, &mut state);

        assert!((cam.position - pos_before).length_squared() < 1e-24, "position must not change with no input");
    }

    #[test]
    fn test_free_fly_forward_movement() {
        let mut cam = FreeFlyCamera::default();
        let pos_before = cam.position;
        let time = SimulationTime::with_dt(1.0);
        let mut input = zero_input();
        input.forward = true;
        let mut state = CameraState::default();

        update_free_fly(&mut cam, &time, &input, &mut state);

        let forward_h = DVec3::new(cam.yaw.cos(), 0.0, cam.yaw.sin()).normalize();
        let expected = pos_before + forward_h * cam.speed * time.dt;
        let diff = (cam.position - expected).length();
        assert!(diff < 1e-12, "forward movement mismatch: {}", diff);
    }

    #[test]
    fn test_free_fly_pitch_clamping() {
        let mut cam = FreeFlyCamera::default();
        let time = default_sim_time();
        let mut input = zero_input();
        input.mouse_delta_y = 10_000.0; // extreme look up
        let mut state = CameraState::default();

        update_free_fly(&mut cam, &time, &input, &mut state);
        assert!(cam.pitch <= 1.5, "pitch must clamp ≤ 1.5, got {}", cam.pitch);

        input.mouse_delta_y = -10_000.0; // extreme look down
        update_free_fly(&mut cam, &time, &input, &mut state);
        assert!(cam.pitch >= -1.5, "pitch must clamp ≥ -1.5, got {}", cam.pitch);
    }

    #[test]
    fn test_free_fly_speed_scaling() {
        let mut cam = FreeFlyCamera::default();
        let time = default_sim_time();
        let mut input = zero_input();
        input.scroll_delta = 1.0;
        let mut state = CameraState::default();

        let speed_before = cam.speed;
        update_free_fly(&mut cam, &time, &input, &mut state);

        let expected = speed_before * 2.0_f64.powf(1.0 * 0.25);
        assert!((cam.speed - expected).abs() < 1e-12, "speed scaling mismatch: expected {}, got {}", expected, cam.speed);
    }

    #[test]
    fn test_free_fly_speed_clamping() {
        let mut cam = FreeFlyCamera::default();
        let time = default_sim_time();
        let mut input = zero_input();
        input.scroll_delta = 1_000.0;
        let mut state = CameraState::default();

        update_free_fly(&mut cam, &time, &input, &mut state);
        assert!(cam.speed <= 1.0e12, "speed must clamp to max");

        input.scroll_delta = -1_000.0;
        update_free_fly(&mut cam, &time, &input, &mut state);
        assert!(cam.speed >= 1.0, "speed must clamp to min, got {}", cam.speed);
    }

    // ── Orbital tests ───────────────────────────────────────────

    #[test]
    fn test_orbital_eye_distance() {
        let cam = OrbitalCamera::default();
        let input = zero_input();
        let mut state = CameraState::default();

        let mut mutable_cam = cam.clone();
        update_orbital(&mut mutable_cam, &input, &mut state);

        let dist = (state.position - mutable_cam.target).length();
        assert!((dist - mutable_cam.distance).abs() < 1e-12,
            "eye-target distance must match: expected {}, got {}", mutable_cam.distance, dist);
    }

    #[test]
    fn test_orbital_phi_clamping() {
        let mut cam = OrbitalCamera::default();
        let mut input = zero_input();
        input.left_mouse = true;
        input.mouse_delta_y = -10_000.0; // drag up aggressively
        let mut state = CameraState::default();

        update_orbital(&mut cam, &input, &mut state);
        assert!(cam.phi > 0.0, "phi must stay above 0, got {}", cam.phi);

        input.mouse_delta_y = 10_000.0; // drag down
        update_orbital(&mut cam, &input, &mut state);
        assert!(cam.phi < std::f64::consts::PI, "phi must stay below PI, got {}", cam.phi);
    }

    #[test]
    fn test_orbital_zoom_clamping() {
        let mut cam = OrbitalCamera::default();
        let mut input = zero_input();
        input.scroll_delta = -10_000.0; // zoom out
        let mut state = CameraState::default();

        update_orbital(&mut cam, &input, &mut state);
        assert!(cam.distance <= cam.max_distance, "distance must clamp to max, got {}", cam.distance);

        input.scroll_delta = 10_000.0; // zoom in
        update_orbital(&mut cam, &input, &mut state);
        assert!(cam.distance >= cam.min_distance, "distance must clamp to min, got {}", cam.distance);
    }

    #[test]
    fn test_orbital_theta_rotation() {
        let mut cam = OrbitalCamera::default();
        let theta_before = cam.theta;
        let mut input = zero_input();
        input.left_mouse = true;
        input.mouse_delta_x = 100.0;
        let mut state = CameraState::default();

        update_orbital(&mut cam, &input, &mut state);
        let expected = theta_before - 100.0 * cam.sensitivity;
        assert!((cam.theta - expected).abs() < 1e-12,
            "theta rotation mismatch: expected {}, got {}", expected, cam.theta);
    }

    // ── State consistency tests ─────────────────────────────────

    #[test]
    fn test_camera_state_orthonormal() {
        let mut cam = FreeFlyCamera::default();
        let time = default_sim_time();
        let mut input = zero_input();
        let mut state = CameraState::default();

        // Simulate a few frames with rotation
        input.mouse_delta_x = 50.0;
        input.mouse_delta_y = 30.0;
        input.forward = true;
        update_free_fly(&mut cam, &time, &input, &mut state);

        assert!((state.forward.length() - 1.0).abs() < 1e-12, "forward must be unit");
        assert!((state.right.length() - 1.0).abs() < 1e-12, "right must be unit");
        assert!((state.up.length() - 1.0).abs() < 1e-12, "up must be unit");
        assert!(state.forward.dot(state.right).abs() < 1e-12, "forward·right must be 0");
        assert!(state.forward.dot(state.up).abs() < 1e-12, "forward·up must be 0");
        assert!(state.right.dot(state.up).abs() < 1e-12, "right·up must be 0");
    }

    #[test]
    fn test_orbital_state_orthonormal() {
        let mut cam = OrbitalCamera::default();
        let mut input = zero_input();
        input.left_mouse = true;
        input.mouse_delta_x = 40.0;
        input.mouse_delta_y = 20.0;
        let mut state = CameraState::default();

        update_orbital(&mut cam, &input, &mut state);

        assert!((state.forward.length() - 1.0).abs() < 1e-12, "forward must be unit");
        assert!((state.right.length() - 1.0).abs() < 1e-12, "right must be unit");
        assert!((state.up.length() - 1.0).abs() < 1e-12, "up must be unit");
        assert!(state.forward.dot(state.right).abs() < 1e-12, "forward·right must be 0");
    }
}
