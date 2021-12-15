struct Pixel {
  r: u32;
  g: u32;
  b: u32;
};

fn pixel_to_vec(p: Pixel) -> vec3<f32> {
  return vec3<f32>(f32(p.r), f32(p.g), f32(p.b)) / 255.0;
}

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
[[group(1), binding(0)]] var<storage, read> color_buffer: ColorBuffer;

struct VertexOutput {
  [[builtin(position)]] pos: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] vertex_idx: u32) -> VertexOutput {
  let uv = vec2<u32>((vertex_idx << 1u) & 2u, vertex_idx & 2u);
  let out = VertexOutput(vec4<f32>(2.0 * vec2<f32>(uv) - 1.0, 0.0, 1.0));
  return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  let x = floor(in.pos.x);
  let y = floor(in.pos.y);
  let index = u32(x + y * screen_dims.screen_width);
  let p = color_buffer.value[index];

  let pixel = pixel_to_vec(p);

  let col = vec4<f32>(pixel, 1.0);
  return col;
}
