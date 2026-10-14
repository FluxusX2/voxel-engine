struct LightCamera {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> light_camera: LightCamera;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    return light_camera.view_projection * vec4<f32>(input.position, 1.0);
}