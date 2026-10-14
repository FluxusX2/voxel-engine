use glam::{camera, Mat4, Vec3};

const FOV_Y: f32 = 90.0_f32.to_radians();
const NEAR_PLANE: f32 = 0.1;
const FAR_PLANE: f32 = 100.0;
const MAX_PITCH: f32 = 89.0_f32.to_radians();

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
    aspect_ratio: f32,
}

impl Camera {
    pub(crate) fn position(&self) -> Vec3 {
        self.pos
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Vec3::new(0.0, 0.0, 3.0),
            yaw: -90.0_f32.to_radians(),
            pitch: 0.0,
            aspect_ratio: 16.0 / 9.0,
        }
    }
}

impl Camera {
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        assert!(aspect_ratio.is_finite() && aspect_ratio > 0.0);
        self.aspect_ratio = aspect_ratio;
    }

    pub fn set_position(&mut self, pos: Vec3) {
        self.pos = pos;
    }

    pub fn rotate(&mut self, yaw_delta: f32, pitch_delta: f32) {
        self.yaw = (self.yaw + yaw_delta).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + pitch_delta).clamp(-MAX_PITCH, MAX_PITCH);
    }

    pub fn view_direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        ).normalize()
    }

    pub fn projection_matrix(&self) -> Mat4 {
        camera::rh::proj::directx::perspective(FOV_Y, self.aspect_ratio, NEAR_PLANE, FAR_PLANE)
    }

    pub fn view_matrix(&self) -> Mat4 {
        camera::rh::view::look_at_mat4(self.pos, self.pos + self.view_direction(), Vec3::Y)
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn move_local(&mut self, forward_input: f32, right_input: f32, up_input: f32, dt: f32) {
        const SPEED: f32 = 5.0;
        let forward = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin());
        let right = forward.cross(Vec3::Y);
        let movement = forward * forward_input + right * right_input + Vec3::Y * up_input;
        self.pos += movement.normalize_or_zero() * SPEED * dt;
    }
}