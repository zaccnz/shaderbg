// Lygia Rand https://github.com/patriciogonzalezvivo/lygia/blob/b68cb6f0f33669f10853ea2b35bd1c4621517f33/generative/random.wgsl
const RANDOM_SINLESS: bool = true;
const RANDOM_SCALE: vec4<f32> = vec4<f32>(.1031, .1030, .0973, .1099);

fn random(p: f32) -> f32 {
    var x = p;
    if RANDOM_SINLESS {
        return fract(sin(x) * 43758.5453);
    } else {
        x = fract(x * RANDOM_SCALE.x);
        x *= x + 33.33;
        x *= x + x;
        return fract(x);
    }
}

fn random2(st: vec2<f32>) -> f32 {
    if RANDOM_SINLESS {
        var p3 = fract(vec3(st.xyx) * RANDOM_SCALE.xyz);
        p3 += dot(p3, p3.yzx + 33.33);
        return fract((p3.x + p3.y) * p3.z);
    } else {
        return fract(sin(dot(st.xy, vec2(12.9898, 78.233))) * 43758.5453);
    }
}

fn random3(p: vec3<f32>) -> f32 {
    var pos = p;
    if RANDOM_SINLESS {
        pos = fract(pos * RANDOM_SCALE.xyz);
        pos += dot(pos, pos.zyx + 31.32);
        return fract((pos.x + pos.y) * pos.z);
    } else {
        return fract(sin(dot(pos.xyz, vec3(70.9898, 78.233, 32.4355))) * 43758.5453123);
    }
}

// Variables

struct CameraMatrix {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
    time: i32,
};

@group(0) @binding(0)
var<uniform> camera: CameraMatrix;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) pos: vec3<f32>,
};

// Vertex shader

const waveSpeed: f32 = 1.0;
const waveHeight: f32 = 15.0;
const waveNoise: f32 = 4.0;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = vec3<f32>(0.0, 0.329, 0.529);

    let v = model.position;

    let crossChop = sqrt(waveSpeed) * cos(-v.x - (v.z * 0.7)); // + s * (i % 229) / 229 * 5
    let t = ((waveSpeed * f32(u32(camera.time)) / (1000.0 / 60.0) * 0.02) - (waveSpeed * v.x * 0.025)) + (waveSpeed * v.z * 0.015);
    let delta = sin(t + crossChop);
    let trochoidDelta = pow(delta + 1.0, 2.0) / 4.0;

    let noise = random3(v) * waveNoise;
    let wave = trochoidDelta * waveHeight;

    let y = v.y + noise + wave;

    let pos = vec4(model.position.x, y, model.position.z, 1.0);

    out.clip_position = camera.proj * camera.view * pos;

    let pos_view = camera.view * pos;
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

    let result = (ambient + diffuse) * in.color;

    return vec4<f32>(result, 1.0);
}