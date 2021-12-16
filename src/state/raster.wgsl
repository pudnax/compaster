struct Pixel { r: f32; g: f32; b: f32; };

[[block]]
struct ColorBuffer {
  values: array<Pixel>;
};

struct Vertex { x: f32; y: f32; z: f32; };

[[block]]
struct VertexBuffer {
  values: array<Vertex>;
};

[[block]]
struct Uniform {
  width: f32;
  height: f32;
};

[[block]]
struct Camera {
  view_pos: vec4<f32>;
  view_proj: mat4x4<f32>;
};

[[group(0), binding(0)]] var<storage, read_write> color_buffer : ColorBuffer;
[[group(1), binding(0)]] var<storage, read> vertex_buffer : VertexBuffer;
[[group(2), binding(0)]] var<uniform> screen_dims : Uniform;
[[group(3), binding(0)]] var<uniform> camera : Camera;

fn project(v: Vertex) -> vec3<f32> {
  var screen_pos = camera.view_proj * vec4<f32>(v.x, v.y, v.z, 1.0);
  screen_pos.x = (screen_pos.x / screen_pos.w) * screen_dims.width;
  screen_pos.y = (screen_pos.y / screen_pos.w) * screen_dims.height;

  return screen_pos.xyw;
}

fn color_pixel(x: u32, y: u32, pixel: Pixel) {
  let pixel_id = x + y * u32(screen_dims.width);
  color_buffer.values[pixel_id] = pixel;
}

fn draw_line(v1: vec3<f32>, v2: vec3<f32>) {
  let dist = i32(distance(v1.xy, v2.xy));
  for (var i = 0; i < dist; i = i + 1) {
    let x = v1.x + (v2.x - v1.x) * (f32(i) / f32(dist));
    let y = v1.y + (v2.y - v1.y) * (f32(i) / f32(dist));
    color_pixel(u32(x), u32(y), Pixel(1.0, 1.0, 1.0));
  }
}

// From: https://github.com/ssloy/tinyrenderer/wiki/Lesson-2:-Triangle-rasterization-and-back-face-culling
fn barycentric(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>, p: vec2<f32>) -> vec3<f32> {
  let u = cross(vec3<f32>(v3.x - v1.x, v2.x - v1.x, v1.x - p.x),
                vec3<f32>(v3.y - v1.y, v2.y - v1.y, v1.y - p.y));

  if (abs(u.z) < 1.0) {
    return vec3<f32>(-1.0, 1.0, 1.0);
  }

  return vec3<f32>(1.0 - (u.x + u.y) / u.z, u.y / u.z, u.x / u.z);
}

fn get_min_max(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) -> vec4<f32> {
  var min_max = vec4<f32>(0.);
  min_max.x = min(min(v1.x, v2.x), v3.x);
  min_max.y = min(min(v1.y, v2.y), v3.y);
  min_max.z = max(max(v1.x, v2.x), v3.x);
  min_max.w = max(max(v1.y, v2.y), v3.y);

  return min_max;
}

fn draw_triangle(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) {
  let min_max = get_min_max(v1, v2, v3);
  let startX = u32(min_max.x);
  let startY = u32(min_max.y);
  let endX = u32(min_max.z);
  let endY = u32(min_max.w);

  for (var x: u32 = startX; x <= endX; x = x + 1u) {
    for (var y : u32 = startY; y <= endY; y = y + 1u) {
      let bc = barycentric(v1, v2, v3, vec2<f32>(f32(x), f32(y)));
      let color = (bc.x * v1.z + bc.y * v2.z + bc.z * v3.z) - 10.;

      let R = color;
      let G = color;
      let B = color;

      if (bc.x < 0.0 || bc.y < 0.0 || bc.z < 0.0) {
        continue;
      }
      color_pixel(x, y, Pixel(R, G, B));
    }
  }
}

// move it inside the color pix function
fn is_off_screen(v: vec3<f32>) -> bool {
  if (v.x < 0.0 || v.x > screen_dims.width || v.y < 0.0 ||
      v.y > screen_dims.height) {
    return true;
  }

  return false;
}

[[stage(compute), workgroup_size(256, 1, 1)]]
fn raster([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
  let index = global_id.x;
  let vertex_idx = index * 3u;

  let v1 = project(vertex_buffer.values[vertex_idx + 0u]);
  let v2 = project(vertex_buffer.values[vertex_idx + 1u]);
  let v3 = project(vertex_buffer.values[vertex_idx + 2u]);

  if (is_off_screen(v1) || is_off_screen(v2) || is_off_screen(v3)) {
    return;
  }

  // color_pixel(u32(v1.x), u32(v1.y), Pixel(1.0, 0.0, 0.0));
  // color_pixel(u32(v2.x), u32(v2.y), Pixel(1.0, 0.0, 0.0));
  // color_pixel(u32(v3.x), u32(v3.y), Pixel(1.0, 0.0, 0.0));

  // draw_line(v1, v2);
  // draw_line(v1, v3);
  // draw_line(v2, v3);

  draw_triangle(v1, v2, v3);
}

[[stage(compute), workgroup_size(256, 1, 1)]]
fn clear([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
  let index = global_id.x;

  color_buffer.values[index] = Pixel(0., 1., 0.333);
}
