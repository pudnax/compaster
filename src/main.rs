mod camera;
mod state;

use camera::Camera;
use glam::vec3;
use state::State;

use std::time::{Duration, Instant};

use color_eyre::Result;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
};

fn main() -> Result<()> {
    env_logger::init();
    color_eyre::install()?;

    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("WGPU - Compute Raster")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)?;
    let PhysicalSize { width, height } = window.inner_size();

    let mut state = pollster::block_on({
        let camera = Camera::new(
            1.5,
            0.5,
            1.25,
            vec3(0.0, 0.0, 0.0),
            width as f32 / height as f32,
        );
        State::new(&window, width, height, camera)
    })?;

    let mut mouse_dragged = false;
    let rotate_speed = 0.0025;
    let zoom_speed = 0.002;

    let mut last_update_inst = Instant::now();
    let mut last_frame_inst = Instant::now();
    let mut frame_counter = FrameCounter::new();
    let time = Instant::now();

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

            Event::DeviceEvent { ref event, .. } => match event {
                DeviceEvent::Button {
                    #[cfg(target_os = "macos")]
                        button: 0,
                    #[cfg(not(target_os = "macos"))]
                        button: 1,

                    state: statee,
                } => {
                    let is_pressed = *statee == ElementState::Pressed;
                    mouse_dragged = is_pressed;
                }
                DeviceEvent::MouseWheel { delta, .. } => {
                    let scroll_amount = -match delta {
                        MouseScrollDelta::LineDelta(_, scroll) => scroll * 1.0,
                        MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                            *scroll as f32
                        }
                    };
                    state.camera.add_zoom(scroll_amount * zoom_speed);
                }
                DeviceEvent::MouseMotion { delta } => {
                    if mouse_dragged {
                        state.camera.add_yaw(-delta.0 as f32 * rotate_speed);
                        state.camera.add_pitch(delta.1 as f32 * rotate_speed);
                    }
                }
                _ => (),
            },

            Event::RedrawRequested(_) => {
                frame_counter.record(&mut last_frame_inst);
                state.update(time.elapsed().as_secs_f32());
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        state.resize(state.width, state.height);
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
