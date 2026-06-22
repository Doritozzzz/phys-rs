use super::buffers::SphUniformData;
use super::GpuContext;
use std::borrow::Cow;
use wgpu::{BindGroupLayout, ComputePipeline};

pub struct SphPipeline {
    pub density_pipeline: ComputePipeline,
    pub pressure_pipeline: ComputePipeline,
    pub force_pipeline: ComputePipeline,
    pub integrate_pipeline: ComputePipeline,
    pub xsph_pipeline: ComputePipeline,
    pub bind_group_layout: BindGroupLayout,
}

pub struct GpuSphBuffers {
    pub position_buffer: wgpu::Buffer,
    pub velocity_buffer: wgpu::Buffer,
    pub density_buffer: wgpu::Buffer,
    pub pressure_buffer: wgpu::Buffer,
    pub force_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub num_particles: u32,
}

impl SphPipeline {
    pub fn new(ctx: &GpuContext) -> Self {
        let shader_source = include_str!("sph.wgsl");

        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sph_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source)),
        });

        let bind_group_layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sph_bind_group_layout"),
            entries: &[
                // binding 0: positions (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: velocities (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 2: densities (read_write)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 3: pressures (read_write)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 4: forces (read_write)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 5: uniforms (read-only, uniform)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sph_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let density_pipeline = ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sph_density_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("compute_density"),
            compilation_options: Default::default(),
            cache: None,
        });

        let pressure_pipeline = ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sph_pressure_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("compute_pressure"),
            compilation_options: Default::default(),
            cache: None,
        });

        let force_pipeline = ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sph_force_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("compute_forces"),
            compilation_options: Default::default(),
            cache: None,
        });

        let integrate_pipeline = ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sph_integrate_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("compute_integrate"),
            compilation_options: Default::default(),
            cache: None,
        });

        let xsph_pipeline = ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sph_xsph_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("compute_xsph"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            density_pipeline,
            pressure_pipeline,
            force_pipeline,
            integrate_pipeline,
            xsph_pipeline,
            bind_group_layout,
        }
    }

    pub fn create_buffers_and_bind_group(
        &self,
        ctx: &GpuContext,
        num_particles: u32,
        positions: &[super::buffers::SphParticleVec4],
        velocities: &[super::buffers::SphParticleVec4],
        uniforms: &SphUniformData,
    ) -> GpuSphBuffers {
        let position_buffer = super::buffers::create_position_buffer(&ctx.device, "sph_positions", positions);
        let velocity_buffer = super::buffers::create_velocity_buffer(&ctx.device, "sph_velocities", velocities);
        let density_buffer = super::buffers::create_density_buffer(&ctx.device, "sph_densities", num_particles);
        let pressure_buffer = super::buffers::create_pressure_buffer(&ctx.device, "sph_pressures", num_particles);
        let force_buffer = super::buffers::create_force_buffer(&ctx.device, "sph_forces", num_particles);
        let uniform_buffer = super::buffers::create_uniform_buffer(&ctx.device, "sph_uniforms", uniforms);

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sph_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: position_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: velocity_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: density_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: pressure_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: force_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        GpuSphBuffers {
            position_buffer,
            velocity_buffer,
            density_buffer,
            pressure_buffer,
            force_buffer,
            uniform_buffer,
            bind_group,
            num_particles,
        }
    }

    pub fn dispatch_all(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        buffers: &GpuSphBuffers,
    ) {
        let workgroups = (buffers.num_particles + 255) / 256;

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sph_density_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.density_pipeline);
            pass.set_bind_group(0, &buffers.bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sph_pressure_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pressure_pipeline);
            pass.set_bind_group(0, &buffers.bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sph_force_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.force_pipeline);
            pass.set_bind_group(0, &buffers.bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sph_integrate_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.integrate_pipeline);
            pass.set_bind_group(0, &buffers.bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sph_xsph_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.xsph_pipeline);
            pass.set_bind_group(0, &buffers.bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }
}
