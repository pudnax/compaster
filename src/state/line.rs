use super::{v, Vertex};
use wgpu::{util::DeviceExt, Device};

#[rustfmt::skip]
pub const VERTICES: [Vertex; 16] = [
    v!(-1.0, -1.0, -1.0), v!( 1.0, -1.0, -1.0),
    v!(-1.0, -1.0, -1.0), v!(-1.0, -1.0,  1.0),
    v!(-1.0, -1.0,  1.0), v!( 1.0, -1.0,  1.0),
    v!( 1.0, -1.0,  1.0), v!( 1.0, -1.0, -1.0),
    v!(-1.0,  1.0, -1.0), v!( 1.0,  1.0, -1.0),
    v!(-1.0,  1.0, -1.0), v!(-1.0,  1.0,  1.0),
    v!(-1.0,  1.0,  1.0), v!( 1.0,  1.0,  1.0),
    v!( 1.0,  1.0,  1.0), v!( 1.0,  1.0, -1.0),
];

pub fn draw_lines_command(
    device: &Device,
    sample_count: u32,
    format: wgpu::TextureFormat,
    camera_uniform: &wgpu::Buffer,
) -> wgpu::RenderBundle {
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let line_shader = device.create_shader_module(&wgpu::include_wgsl!("line.wgsl"));
    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raster: Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Line Cam"),
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_uniform.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Line Pipeline Layout"),
        bind_group_layouts: &[&camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Line Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &line_shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: Vertex::SIZE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &Vertex::ATTR,
            }],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            ..Default::default()
        },
        depth_stencil: None,
        // depth_stencil: Some(wgpu::DepthStencilState {
        //     format: wgpu::TextureFormat::Depth32Float,
        //     depth_write_enabled: true,
        //     depth_compare: wgpu::CompareFunction::Less,
        //     stencil: wgpu::StencilState::default(),
        //     bias: wgpu::DepthBiasState::default(),
        // }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: true,
        },
        fragment: Some(wgpu::FragmentState {
            module: &line_shader,
            entry_point: "fs_main",
            targets: &[format.into()],
        }),
        multiview: None,
    });

    let mut encoder = device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
        label: Some("Line Bundle Encoder"),
        color_formats: &[format],
        depth_stencil: None,
        sample_count,
        multiview: None,
    });
    encoder.set_pipeline(&line_pipeline);
    encoder.set_bind_group(0, &camera_bind_group, &[]);
    encoder.set_vertex_buffer(0, vertex_buffer.slice(..));
    encoder.draw(0..VERTICES.len() as _, 0..1);
    encoder.finish(&wgpu::RenderBundleDescriptor {
        label: Some("Draw Lines Bundle"),
    })
}
