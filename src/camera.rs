use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_position: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = [camera.eye.x, camera.eye.y, camera.eye.z, 1.0];
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub zoom: f32,
    pub target: Vec3,
    pub eye: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub up: Vec3,
    pub aspect: f32,
}

impl Camera {
    const ZFAR: f32 = 100.;
    const ZNEAR: f32 = 0.1;
    const FOVY: f32 = std::f32::consts::PI / 2.0;
    const UP: Vec3 = Vec3::Y;

    pub fn new(zoom: f32, pitch: f32, yaw: f32, target: Vec3, aspect: f32) -> Self {
        let mut camera = Self {
            zoom,
            pitch,
            yaw,
            eye: Vec3::ZERO,
            target,
            up: Self::UP,
            aspect,
        };
        camera.update();
        camera
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        // let view = view * Mat4::from_translation(glam::vec3(4., 3., -10.));
        let proj = Mat4::perspective_rh(Self::FOVY, self.aspect, Self::ZNEAR, Self::ZFAR);
        proj * view
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.3, Self::ZFAR / 2.);
        self.update();
    }

    pub fn add_zoom(&mut self, delta: f32) {
        self.set_zoom(self.zoom + delta);
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch.clamp(
            -std::f32::consts::PI / 2.0 + f32::EPSILON,
            std::f32::consts::PI / 2.0 - f32::EPSILON,
        );
        self.update();
    }

    pub fn add_pitch(&mut self, delta: f32) {
        self.set_pitch(self.pitch + delta);
    }

    pub fn set_yaw(&mut self, yaw: f32) {
        self.yaw = yaw;
        self.update();
    }

    pub fn add_yaw(&mut self, delta: f32) {
        self.set_yaw(self.yaw + delta);
    }

    fn update(&mut self) {
        let pitch_cos = self.pitch.cos();
        self.eye = self.zoom
            * Vec3::new(
                self.yaw.sin() * pitch_cos,
                self.pitch.sin(),
                self.yaw.cos() * pitch_cos,
            );
    }
}
