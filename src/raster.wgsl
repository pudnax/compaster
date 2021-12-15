struct Pixel {
  r: f32;
  g: f32;
  b: f32;
};

[[block]]
struct ColorBuffer {
  value: array<Pixel>;
};

[[block]]
struct Uniform {
  screen_width: f32;
  screen_height: f32;
};

[[group(0), binding(0)]] var<uniform> screen_dims : Uniform;
[[group(1), binding(0)]] var<storage, read_write> color_buffer: ColorBuffer;

[[stage(compute), workgroup_size(256, 1, 1)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
  let index = global_id.x;
  let buf = &color_buffer.value[index];

  (*buf) = Pixel(0.15, 0.0, 0.1);
}
