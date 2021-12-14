use std::{
    ops::Deref,
    time::{Duration, Instant},
};

use color_eyre::{owo_colors::OwoColorize, Result};
use raw_window_handle::HasRawWindowHandle;
use wgpu::SurfaceConfiguration;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct State {
    device: wgpu::Device,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
    queue: wgpu::Queue,
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

        Ok(Self {
            device,
            surface,
            surface_config,
            queue,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
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
        let _ = &state;
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
                _ => {}
            },
            Event::RedrawRequested(_) => {
                frame_counter.record(&mut last_frame_inst);
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
