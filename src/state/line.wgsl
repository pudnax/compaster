[[block]]
struct Camera {
  view_pos: vec4<f32>;
  view_proj: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> camera: Camera;

struct VertexOutput {
  [[builtin(position)]] clip_position: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main([[location(0)]] position: vec3<f32>) -> VertexOutput {
  return VertexOutput(camera.view_proj * vec4<f32>(position, 1.0));
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  return vec4<f32>(1.);
}
