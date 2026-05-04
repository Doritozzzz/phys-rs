use encase::{ShaderType, StorageBuffer, ShaderSize, internal::WriteInto};
use wgpu::util::DeviceExt;



/// Creates a new GPU storage buffer containing the given slice of data.
pub fn create_storage_buffer<T: ShaderType + Clone + ShaderSize + WriteInto>(
    device: &wgpu::Device,
    label: &str,
    data: &[T],
) -> wgpu::Buffer {
    let mut buffer = StorageBuffer::new(Vec::new());
    buffer.write(&data.to_vec()).unwrap();
    let bytes = buffer.into_inner();

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: &bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
    })
}

/// Creates an empty uninitialized GPU storage buffer of a specific size.
pub fn create_empty_storage_buffer(
    device: &wgpu::Device,
    label: &str,
    size_bytes: u64,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}
