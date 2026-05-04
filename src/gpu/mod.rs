use bevy_ecs::prelude::*;
use std::sync::Arc;
use wgpu::{Adapter, Device, Instance, InstanceDescriptor, PowerPreference, Queue};

pub mod buffers;
pub mod pipeline;

#[derive(Resource, Clone)]
pub struct GpuContext {
    pub instance: Arc<Instance>,
    pub adapter: Arc<Adapter>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
}

#[derive(Resource)]
pub struct GpuConfig {
    pub prefer_high_performance: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            prefer_high_performance: true, // Default to HighPerformance per user request
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
    
    // We don't know if block_on returns Option or Result, but we can match it out.
    // Or just use unwrap for now so we can see the real type if it fails.
    let adapter = pollster::block_on(adapter_future).ok()?;

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

    Some(GpuContext {
        instance: Arc::new(instance),
        adapter: Arc::new(adapter),
        device: Arc::new(device),
        queue: Arc::new(queue),
    })
}
