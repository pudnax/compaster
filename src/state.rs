use bytemuck::{Pod, Zeroable};
use color_eyre::Result;
use raw_window_handle::HasRawWindowHandle;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    SurfaceConfiguration,
};

mod present_pass;
mod raster_pass;

use present_pass::{PresentBindings, PresentPass};
use raster_pass::{RasterBindings, RasterPass};

pub(crate) const WORKGROUP_SIZE: u32 = 256;
pub const fn dispatch_size(len: u32) -> u32 {
    let subgroup_size = WORKGROUP_SIZE;
    let padded_size = (subgroup_size - len % subgroup_size) % subgroup_size;
    (len + padded_size) / subgroup_size
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Uniform {
    screen_width: f32,
    screen_height: f32,
}

impl Uniform {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
        }
    }

    pub fn update(queue: &wgpu::Queue, buffer: &wgpu::Buffer, width: f32, height: f32) {
        queue.write_buffer(buffer, 0, bytemuck::bytes_of(&Uniform::new(width, height)));
    }
}

fn create_color_buffer(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Buffer {
    use std::mem::size_of;
    #[repr(C)]
    struct Pixel {
        r: f32,
        g: f32,
        b: f32,
    }
    assert!(size_of::<Pixel>() == size_of::<[f32; 3]>());

    let pixel_size = size_of::<Pixel>() as u64;
    let (width, height) = (width as u64, height as u64);
    let size = pixel_size * width * height;

    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    })
}

pub struct State {
    device: wgpu::Device,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
    queue: wgpu::Queue,

    pub width: u32,
    pub height: u32,

    screen_uniform: wgpu::Buffer,
    output_buffer: wgpu::Buffer,

    raster_pass: RasterPass,
    raster_bindings: RasterBindings,

    present_pass: PresentPass,
    present_bindings: PresentBindings,
}

impl State {
    pub async fn new(window: &impl HasRawWindowHandle, width: u32, height: u32) -> Result<Self> {
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

        let output_buffer = create_color_buffer(&device, width, height);

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

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        Uniform::update(&self.queue, &self.screen_uniform, width as _, height as _);

        self.output_buffer = create_color_buffer(&self.device, width, height);
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

    pub fn render(&self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = &frame.texture.create_view(&Default::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
            });

            self.raster_pass.record(
                &mut cpass,
                &self.raster_bindings,
                dispatch_size(self.width * self.height),
            );
        }

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 1.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            self.present_pass.record(&mut rpass, &self.present_bindings);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }
}
