[[block]]
struct ColorBuffer {
  data: array<u32>;
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
  var pos = array<vec2<f32>, 6>(vec2<f32>(1.0, 1.0), vec2<f32>(1.0, -1.0),
                                vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0),
                                vec2<f32>(-1.0, -1.0), vec2<f32>(-1.0, 1.0));

  let out = VertexOutput(vec4<f32>(pos[vertex_idx], 0.0, 1.0));
  return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  let x = floor(in.pos.x);
  let y = floor(in.pos.y);
  let index = u32(x + y * screen_dims.screen_width) * 3u;

  let r = f32(color_buffer.data[index + 0u]) / 255.0;
  let g = f32(color_buffer.data[index + 1u]) / 255.0;
  let b = f32(color_buffer.data[index + 2u]) / 255.0;

  let col = vec4<f32>(r, g, b, 1.0);
  return col;
}
