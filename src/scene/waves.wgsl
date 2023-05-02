// Vertex and fragment shaders to render our wave

struct CameraMatrix {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
};

struct WaveRenderParams {
    colour: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraMatrix;
@group(0) @binding(1)
var<uniform> wave_render_params: WaveRenderParams;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos: vec3<f32>,
};

// Vertex shader

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    let pos = vec4(model.position.x, model.position.y, model.position.z, 1.0);
    let pos_view = camera.view * pos;

    out.clip_position = camera.proj * camera.view * pos;
    out.pos = vec3(pos_view.x, pos_view.y, pos_view.z) / pos_view.w;

    return out;
}

// Fragment shader

const lightPos: vec3<f32> = vec3<f32>(-100.0, 250.0, -100.0);
const lightColor: vec3<f32> = vec3<f32>(0.9, 0.9, 0.9);
const ambient: vec3<f32> = vec3<f32>(0.3, 0.3, 0.3);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let posv3 = vec3(in.clip_position.x, in.clip_position.y, in.clip_position.z);

    let dx = dpdx(in.pos);
    let dy = dpdy(in.pos);
    let N = normalize(cross(dx, dy));


    let lightDir = normalize(lightPos - posv3);
    let diff = max(dot(N, lightDir), 0.0);
    let diffuse = diff * lightColor;

    let result = (ambient + diffuse) * wave_render_params.colour;

    return vec4<f32>(result, 1.0);
}