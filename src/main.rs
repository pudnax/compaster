use std::time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};
use color_eyre::Result;
use raw_window_handle::HasRawWindowHandle;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    SurfaceConfiguration,
};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

const NUM_CHANNELS: u64 = 3;
const WORKGROUP_SIZE: u32 = 256;
pub const fn dispatch_size(len: u32) -> u32 {
    let subgroup_size = WORKGROUP_SIZE;
    let padded_size = (subgroup_size - len % subgroup_size) % subgroup_size;
    (len + padded_size) / subgroup_size
}

struct State {
    device: wgpu::Device,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
    queue: wgpu::Queue,

    width: u32,
    height: u32,

    screen_uniform: wgpu::Buffer,
    output_buffer: wgpu::Buffer,

    raster_pass: RasterPass,
    raster_bindings: RasterBindings,

    present_pass: PresentPass,
    present_bindings: PresentBindings,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniform {
    screen_width: f32,
    screen_height: f32,
}

impl Uniform {
    fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
        }
    }

    fn update(queue: &wgpu::Queue, buffer: &wgpu::Buffer, width: f32, height: f32) {
        queue.write_buffer(buffer, 0, bytemuck::bytes_of(&Uniform::new(width, height)));
    }
}

impl State {
    async fn new(window: &impl HasRawWindowHandle, width: u32, height: u32) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let device_info = adapter.get_info();
        println!("Backend: {:?}", device_info.backend);
        println!("Device Name: {}", device_info.name);
        println!("Device Type: {:?}", device_info.device_type);

        let limits = adapter.limits();
        let features = adapter.features();
        let format = surface.get_preferred_format(&adapter).unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    features,
                    limits,
                },
                None,
            )
            .await?;

        let surface_config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Immediate,
        };

        surface.configure(&device, &surface_config);

        let present_pass = PresentPass::new(&device, format);
        let raster_pass = RasterPass::new(&device);

        let screen_uniform = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Screen Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&Uniform::new(width as _, height as _)),
        });

        let output_buffer = {
            let size =
                std::mem::size_of::<f32>() as u64 * width as u64 * height as u64 * NUM_CHANNELS;

            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Output Buffer"),
                size,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };

        let present_bindings = PresentBindings::new(
            &device,
            &present_pass.pipeline,
            &screen_uniform,
            &output_buffer,
        );
        let raster_bindings = RasterBindings::new(
            &device,
            &raster_pass.pipeline,
            &screen_uniform,
            &output_buffer,
        );

        Ok(Self {
            device,
            surface,
            surface_config,
            queue,

            width,
            height,

            screen_uniform,

            output_buffer,

            raster_pass,
            raster_bindings,

            present_pass,
            present_bindings,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        Uniform::update(&self.queue, &self.screen_uniform, width as _, height as _);

        self.output_buffer = {
            let size =
                std::mem::size_of::<f32>() as u64 * width as u64 * height as u64 * NUM_CHANNELS;

            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Output Buffer"),
                size,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        self.present_bindings = PresentBindings::new(
            &self.device,
            &self.present_pass.pipeline,
            &self.screen_uniform,
            &self.output_buffer,
        );
        self.raster_bindings = RasterBindings::new(
            &self.device,
            &self.raster_pass.pipeline,
            &self.screen_uniform,
            &self.output_buffer,
        );
    }

    fn render(&self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = &frame.texture.create_view(&Default::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
        });
        self.raster_pass.record(
            &mut cpass,
            &self.raster_bindings,
            dispatch_size(self.width * self.height),
        );
        drop(cpass);

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        self.present_pass.record(&mut rpass, &self.present_bindings);
        drop(rpass);

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }
}

struct PresentPass {
    pipeline: wgpu::RenderPipeline,
}

impl PresentPass {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform_bind_group =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Present: Uniform Bind Group Layout"),
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
        let output_color_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Present: Output Buffer Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Present Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group, &output_color_bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(&wgpu::include_wgsl!("present.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Present Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[format.into()],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self { pipeline }
    }
}

struct PresentBindings {
    uniform: wgpu::BindGroup,
    color_buffer: wgpu::BindGroup,
}

impl PresentBindings {
    fn new(
        device: &wgpu::Device,
        pipeline: &wgpu::RenderPipeline,
        uniform: &wgpu::Buffer,
        color_buffer: &wgpu::Buffer,
    ) -> Self {
        let uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Present: Uniform Bind Group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let color_buffer = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Present: Output Buffer Bind Group"),
            layout: &pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: color_buffer.as_entire_binding(),
            }],
        });
        Self {
            uniform,
            color_buffer,
        }
    }
}

impl<'a> PresentPass {
    fn record<'pass>(&'a self, rpass: &mut wgpu::RenderPass<'pass>, bindings: &'a PresentBindings)
    where
        'a: 'pass,
    {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &bindings.uniform, &[]);
        rpass.set_bind_group(1, &bindings.color_buffer, &[]);
        rpass.draw(0..3, 0..1);
    }
}

struct RasterPass {
    pipeline: wgpu::ComputePipeline,
}

impl RasterPass {
    fn new(device: &wgpu::Device) -> Self {
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raster Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group, &output_color_bind_group_layout],
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

struct RasterBindings {
    uniform: wgpu::BindGroup,
    color_buffer: wgpu::BindGroup,
}

impl RasterBindings {
    fn new(
        device: &wgpu::Device,
        pipeline: &wgpu::ComputePipeline,
        uniform: &wgpu::Buffer,
        color_buffer: &wgpu::Buffer,
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
        Self {
            uniform,
            color_buffer,
        }
    }
}

impl<'a> RasterPass {
    fn record<'pass>(
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
        cpass.dispatch(dispatch_size, 1, 1);
    }
}

fn main() -> Result<()> {
    env_logger::init();
    color_eyre::install()?;

    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("WGPU - Compute Raster")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)?;
    let PhysicalSize { width, height } = window.inner_size();

    let mut state = pollster::block_on(State::new(&window, width, height))?;

    let mut last_update_inst = Instant::now();
    let mut last_frame_inst = Instant::now();
    let mut frame_counter = FrameCounter::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::RedrawEventsCleared => {
                let target_frametime = Duration::from_secs_f64(1.0 / 60.0);
                let time_since_last_frame = last_update_inst.elapsed();
                if time_since_last_frame >= target_frametime {
                    window.request_redraw();
                    last_update_inst = Instant::now();
                } else {
                    *control_flow = ControlFlow::WaitUntil(
                        Instant::now() + target_frametime - time_since_last_frame,
                    );
                }
            }

            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    state.resize(size.width, size.height);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    state.resize(new_inner_size.width, new_inner_size.height);
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                frame_counter.record(&mut last_frame_inst);
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        state.resize(state.surface_config.width, state.surface_config.height);
                        window.request_redraw();
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => {
                        eprintln!("{:?}", e);
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    });
}

struct FrameCounter {
    frame_count: u32,
    accum_time: f32,
}

impl FrameCounter {
    fn new() -> Self {
        Self {
            frame_count: 0,
            accum_time: 0.,
        }
    }

    fn record(&mut self, current_instant: &mut Instant) -> f32 /* dt */ {
        self.accum_time += current_instant.elapsed().as_secs_f32();
        *current_instant = Instant::now();
        self.frame_count += 1;
        if self.frame_count == 100 {
            println!(
                "Avg frame time {}ms",
                self.accum_time * 1000.0 / self.frame_count as f32
            );
            self.accum_time = 0.0;
            self.frame_count = 0;
        }
        self.accum_time
    }
}
