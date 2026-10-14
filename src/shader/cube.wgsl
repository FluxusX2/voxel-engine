struct Camera {
    view_projection: mat4x4<f32>,
};

struct Lighting {
    direction: vec4<f32>,
    color: vec4<f32>,
    ground_ambient: vec4<f32>,
    sky_ambient: vec4<f32>,
};

struct LightCamera {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var<uniform> lighting: Lighting;

@group(0) @binding(2)
var<uniform> light_camera: LightCamera;

@group(1) @binding(0)
var shadow_map: texture_depth_2d;

@group(1) @binding(1)
var shadow_sampler: sampler_comparison;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) ao: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) ao: f32,
    @location(3) light_position: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_position = vec4<f32>(input.position, 1.0);

    output.clip_position = camera.view_projection * world_position;
    output.light_position = light_camera.view_projection * world_position;
    output.color = input.color;
    output.normal = input.normal;
    output.ao = input.ao;

    return output;
}

fn shadow_visibility(light_position: vec4<f32>) -> f32 {
    let ndc = light_position.xyz / light_position.w;
    let uv = vec2<f32>(
        ndc.x * 0.5 + 0.5,
        0.5 - ndc.y * 0.5,
    );

    if uv.x < 0.0 || uv.x > 1.0 ||
       uv.y < 0.0 || uv.y > 1.0 ||
       ndc.z < 0.0 || ndc.z > 1.0 {
        return 1.0;
    }

    let texel_size = 1.0 / vec2<f32>(textureDimensions(shadow_map));
    let compare_depth = ndc.z - 0.001;

    var visibility = 0.0;

    for (var y = -2; y <= 2; y += 1) {
        for (var x = -2; x <= 2; x += 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            visibility += textureSampleCompare(
                shadow_map,
                shadow_sampler,
                uv + offset,
                compare_depth,
            );
        }
    }

    return visibility / 25.0;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(input.normal);
    let light_direction = normalize(lighting.direction.xyz);
    let diffuse = max(dot(n, light_direction), 0.0);
    let visibility = shadow_visibility(input.light_position);

    let sky_amount = clamp(n.y * 0.5 + 0.5, 0.0, 1.0);
    let ambient = mix(
        lighting.ground_ambient.rgb,
        lighting.sky_ambient.rgb,
        sky_amount,
    );

    let illumination =
        ambient * input.ao
        + lighting.color.rgb * diffuse * visibility;

    return vec4<f32>(input.color * illumination, 1.0);
}