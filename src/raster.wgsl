[[block]]
struct ColorBuffer {
  value: array<u32>;
 };

[[block]]
struct Uniform {
  screen_width: f32;
  screen_height: f32;
};

[[group(0), binding(0)]] var<uniform> screen_dims : Uniform;
// write?
[[group(1), binding(0)]] var<storage, read_write> color_buffer: ColorBuffer;

[[stage(compute), workgroup_size(256, 1, 1)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
  let index = global_id.x * 3u;

  color_buffer.value[index + 0u] = 25u;
  color_buffer.value[index + 1u] = 1u;
  color_buffer.value[index + 2u] = 1u;
}
