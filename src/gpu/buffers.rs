use encase::{ShaderType, StorageBuffer, UniformBuffer};
use glam::DVec3;
use wgpu::util::DeviceExt;

/// SPH particle data for GPU (position + velocity as vec4 for alignment).
#[derive(ShaderType, Clone, Copy, Debug)]
pub struct SphParticleVec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl From<DVec3> for SphParticleVec4 {
    fn from(v: DVec3) -> Self {
        Self {
            x: v.x as f32,
            y: v.y as f32,
            z: v.z as f32,
            w: 0.0,
        }
    }
}

/// SPH uniform parameters matching WGSL `SphParams` struct.
#[derive(ShaderType, Clone, Copy, Debug)]
pub struct SphUniformData {
    pub num_particles: u32,
    pub mass: f32,
    pub h: f32,
    pub dt: f32,
    pub rest_density: f32,
    pub speed_of_sound: f32,
    pub gamma: f32,
    pub alpha_visc: f32,
    pub beta_visc: f32,
    pub _padding: u32,
}

impl SphUniformData {
    pub fn new(
        num_particles: u32,
        mass: f32,
        h: f32,
        dt: f32,
        rest_density: f32,
        speed_of_sound: f32,
        gamma: f32,
        alpha_visc: f32,
        beta_visc: f32,
    ) -> Self {
        Self {
            num_particles,
            mass,
            h,
            dt,
            rest_density,
            speed_of_sound,
            gamma,
            alpha_visc,
            beta_visc,
            _padding: 0,
        }
    }
}

/// Create a GPU storage buffer from particle positions.
pub fn create_position_buffer(
    device: &wgpu::Device,
    label: &str,
    positions: &[SphParticleVec4],
) -> wgpu::Buffer {
    let mut storage = StorageBuffer::new(Vec::new());
    storage.write(positions).unwrap();
    let bytes = storage.into_inner();
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: &bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    })
}

/// Create a GPU storage buffer from particle velocities.
pub fn create_velocity_buffer(
    device: &wgpu::Device,
    label: &str,
    velocities: &[SphParticleVec4],
) -> wgpu::Buffer {
    let mut storage = StorageBuffer::new(Vec::new());
    storage.write(velocities).unwrap();
    let bytes = storage.into_inner();
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: &bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    })
}

/// Create an uninitialized density buffer (read_write).
pub fn create_density_buffer(device: &wgpu::Device, label: &str, count: u32) -> wgpu::Buffer {
    let size = (count as u64) * 4;
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

/// Create an uninitialized pressure buffer (read_write).
pub fn create_pressure_buffer(device: &wgpu::Device, label: &str, count: u32) -> wgpu::Buffer {
    let size = (count as u64) * 4;
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

/// Create a GPU storage buffer for force output (read_write).
pub fn create_force_buffer(
    device: &wgpu::Device,
    label: &str,
    count: u32,
) -> wgpu::Buffer {
    let size = (count as u64) * 16; // vec4<f32> = 16 bytes
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

/// Create a uniform buffer from SphUniformData.
pub fn create_uniform_buffer(
    device: &wgpu::Device,
    label: &str,
    data: &SphUniformData,
) -> wgpu::Buffer {
    let mut uniform = UniformBuffer::new(Vec::new());
    uniform.write(data).unwrap();
    let bytes = uniform.into_inner();
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: &bytes,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

/// Read back densities from GPU to CPU.
pub async fn read_densities(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    count: u32,
) -> Vec<f32> {
    let size = (count as u64) * 4;
    let buffer_slice = buffer.slice(..size);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });
    let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
    receiver.recv().unwrap().unwrap();
    let data = buffer_slice.get_mapped_range();
    let result: Vec<f32> = data
        .chunks_exact(4)
        .map(|b| f32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    drop(data);
    buffer.unmap();
    result
}

/// Read back forces from GPU to CPU.
pub async fn read_forces(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    count: u32,
) -> Vec<[f32; 3]> {
    let size = (count as u64) * 16;
    let buffer_slice = buffer.slice(..size);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });
    let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
    receiver.recv().unwrap().unwrap();
    let data = buffer_slice.get_mapped_range();
    let result: Vec<[f32; 3]> = data
        .chunks_exact(16)
        .map(|b| {
            let x = f32::from_ne_bytes([b[0], b[1], b[2], b[3]]);
            let y = f32::from_ne_bytes([b[4], b[5], b[6], b[7]]);
            let z = f32::from_ne_bytes([b[8], b[9], b[10], b[11]]);
            [x, y, z]
        })
        .collect();
    drop(data);
    buffer.unmap();
    result
}
