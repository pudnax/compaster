pub struct RasterPass {
    pipeline: wgpu::ComputePipeline,
}

impl RasterPass {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_bind_group =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Raster: Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let output_color_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Raster: Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let vertex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Raster: Vertex Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raster Pipeline Layout"),
            bind_group_layouts: &[
                &uniform_bind_group,
                &output_color_bind_group_layout,
                &vertex_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(&wgpu::include_wgsl!("raster.wgsl"));
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Raster Pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: "main",
        });
        Self { pipeline }
    }
}

pub struct RasterBindings {
    uniform: wgpu::BindGroup,
    pub color_buffer: wgpu::BindGroup,
    vertex_buffer: wgpu::BindGroup,
}

impl RasterBindings {
    pub fn new(
        device: &wgpu::Device,
        RasterPass { pipeline }: &RasterPass,
        uniform: &wgpu::Buffer,
        color_buffer: &wgpu::Buffer,
        vertex_buffer: &wgpu::Buffer,
    ) -> Self {
        let uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster: Uniform Bind Group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let color_buffer = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster: Output Buffer Bind Group"),
            layout: &pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: color_buffer.as_entire_binding(),
            }],
        });
        let vertex_buffer = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster: Output Buffer Bind Group"),
            layout: &pipeline.get_bind_group_layout(2),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vertex_buffer.as_entire_binding(),
            }],
        });
        Self {
            uniform,
            color_buffer,
            vertex_buffer,
        }
    }

    pub fn update_color_buffer(
        &mut self,
        device: &wgpu::Device,
        RasterPass { pipeline }: &RasterPass,
        color_buffer: &wgpu::Buffer,
    ) {
        self.color_buffer = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster: Output Buffer Bind Group"),
            layout: &pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: color_buffer.as_entire_binding(),
            }],
        });
    }
}

impl<'a> RasterPass {
    pub fn record<'pass>(
        &'a self,
        cpass: &mut wgpu::ComputePass<'pass>,
        bindings: &'a RasterBindings,
        dispatch_size: u32,
    ) where
        'a: 'pass,
    {
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &bindings.uniform, &[]);
        cpass.set_bind_group(1, &bindings.color_buffer, &[]);
        cpass.set_bind_group(2, &bindings.vertex_buffer, &[]);
        cpass.dispatch(dispatch_size, 1, 1);
    }
}
