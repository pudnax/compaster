use bytemuck::{Pod, Zeroable};

#[allow(clippy::iter_nth_zero)]
pub fn process_model() -> Vec<Vertex> {
    let (model, buffers, _) = {
        let bytes = include_bytes!("../../models/suzanne.glb");
        gltf::import_slice(bytes).unwrap()
    };
    let mesh = model.meshes().nth(0).unwrap();
    let primitives = mesh.primitives().nth(0).unwrap();
    let reader = primitives.reader(|buffer| Some(&buffers[buffer.index()]));
    let positions = reader.read_positions().unwrap().collect::<Vec<_>>();
    reader
        .read_indices()
        .unwrap()
        .into_u32()
        .map(|i| Vertex::from(positions[i as usize]))
        .collect()
}

pub(crate) const WORKGROUP_SIZE: u32 = 256;
pub(crate) const fn dispatch_size(len: u32) -> u32 {
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
}

pub fn create_color_buffer(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Buffer {
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

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    v: [f32; 3],
}

#[allow(dead_code)]
impl Vertex {
    pub const SIZE: u64 = std::mem::size_of::<Self>() as _;
    pub const ATTR: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { v: [x, y, z] }
    }
}

macro_rules! v {
    ($x:expr, $y:expr, $z:expr) => {
        Vertex::new($x, $y, $z)
    };
}
pub(crate) use v;

impl From<[f32; 3]> for Vertex {
    fn from(v: [f32; 3]) -> Self {
        v!(v[0], v[1], v[2])
    }
}

#[allow(dead_code)]
pub const TRIG: [Vertex; 3] = [v!(0.0, 0.5, 0.0), v!(-0.5, 0.0, 0.0), v!(0.5, 0.0, 0.0)];
