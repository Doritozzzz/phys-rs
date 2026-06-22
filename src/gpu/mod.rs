use bevy_ecs::prelude::*;
use std::sync::Arc;
use wgpu::{Adapter, Device, Instance, InstanceDescriptor, PowerPreference, Queue};

pub mod buffers;
pub mod pipeline;

use pipeline::SphPipeline;

#[derive(Resource, Clone)]
pub struct GpuContext {
    pub instance: Arc<Instance>,
    pub adapter: Arc<Adapter>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub adapter_info: GpuAdapterInfo,
}

#[derive(Resource)]
pub enum SphMode {
    Cpu,
    Gpu(GpuContext, SphPipeline),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GpuKind {
    Discrete,
    Integrated,
    Cpu,
    Virtual,
    Other,
    None,
}

#[derive(Resource, Debug, Clone)]
pub struct GpuAdapterInfo {
    pub name: String,
    pub kind: GpuKind,
    pub device_type: String,
    pub backend: String,
    pub has_f64: bool,
}

impl Default for GpuAdapterInfo {
    fn default() -> Self {
        Self {
            name: "none".into(),
            kind: GpuKind::None,
            device_type: "none".into(),
            backend: "none".into(),
            has_f64: false,
        }
    }
}

#[derive(Resource)]
pub struct GpuConfig {
    pub prefer_high_performance: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            prefer_high_performance: true,
        }
    }
}

pub fn initialize_gpu_context(config: &GpuConfig) -> Option<GpuContext> {
    let instance = Instance::new(InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        flags: wgpu::InstanceFlags::default(),
        memory_budget_thresholds: Default::default(),
        backend_options: Default::default(),
        display: Default::default(),
    });

    let power_preference = if config.prefer_high_performance {
        PowerPreference::HighPerformance
    } else {
        PowerPreference::LowPower
    };

    let adapter_future = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference,
        compatible_surface: None,
        force_fallback_adapter: false,
    });

    let adapter = pollster::block_on(adapter_future).ok()?;
    let raw_info = adapter.get_info();
    let kind = match raw_info.device_type {
        wgpu::DeviceType::DiscreteGpu => GpuKind::Discrete,
        wgpu::DeviceType::IntegratedGpu => GpuKind::Integrated,
        wgpu::DeviceType::Cpu => GpuKind::Cpu,
        wgpu::DeviceType::VirtualGpu => GpuKind::Virtual,
        _ => GpuKind::Other,
    };
    let adapter_info = GpuAdapterInfo {
        name: raw_info.name.clone(),
        kind,
        device_type: format!("{:?}", raw_info.device_type),
        backend: format!("{:?}", raw_info.backend),
        has_f64: false,
    };

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("phys_rs_compute_device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            experimental_features: Default::default(),
            trace: Default::default(),
        }
    )).ok()?;

    println!(
        "GPU initialized: {} (backend: {:?}, type: {:?})",
        adapter_info.name, adapter_info.backend, adapter_info.device_type
    );

    Some(GpuContext {
        instance: Arc::new(instance),
        adapter: Arc::new(adapter),
        device: Arc::new(device),
        queue: Arc::new(queue),
        adapter_info,
    })
}

/// ECS system: attempt GPU init at startup.
pub fn try_init_gpu(mut commands: Commands, config: Res<GpuConfig>) {
    if let Some(ctx) = initialize_gpu_context(&config) {
        let pipeline = SphPipeline::new(&ctx);
        commands.insert_resource(SphMode::Gpu(ctx, pipeline));
        println!("SPH GPU mode enabled.");
    } else {
        commands.insert_resource(SphMode::Cpu);
        println!("No compatible GPU found. SPH CPU fallback active.");
    }
}
