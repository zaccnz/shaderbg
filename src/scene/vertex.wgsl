// Compute shader to generate vertices of our wave

const dim_x: i32 = 100;
const dim_y: i32 = 80;

struct WaveParams {
    size: f32,
    speed: f32,
    height: f32,
    noise: f32,
    time: u32,
}

@group(0) @binding(0)
var<uniform> param : WaveParams;

struct Vertex {
    x: f32,
    y: f32,
    z: f32,
}

@group(0) @binding(1)
var<storage, read_write> vertices: array<Vertex>;

fn gen_vertex(x: u32, z: u32) -> Vertex {
    let vx = (f32(x) - (f32(dim_x) * 0.5)) * param.size;
    let vz = ((f32(dim_y) * 0.5) - f32(z)) * param.size;

    let crossChop = sqrt(param.speed) * cos(-vx - (vz * 0.7)); // + s * (i % 229) / 229 * 5
    let t = ((param.speed * f32(param.time) / (1000.0 / 60.0) * 0.02) - (param.speed * vx * 0.025)) + (param.speed * vz * 0.015);
    let delta = sin(t + crossChop);
    let trochoidDelta = pow(delta + 1.0, 2.0) / 4.0;

    let noise = random3(vec3<f32>(vx, -10.0, vz)) * param.noise;
    let wave = trochoidDelta * param.height;

    let vy = -10.0 + noise + wave;

    return Vertex(vx, vy, vz);
}

@compute
@workgroup_size(1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // only care about x,y of global_id
    let x = global_id.x;
    let y = global_id.y;

    let a = gen_vertex(x + u32(1), y + u32(1));
    let b = gen_vertex(x + u32(1), y);
    let c = gen_vertex(x, y + u32(1));
    let d = gen_vertex(x, y);

    let index = ((x * u32(dim_y)) + y) * u32(6);

    if random2(vec2<f32>(a.x, a.z)) > 0.5 {
        vertices[index] = a;
        vertices[index + u32(1)] = b;
        vertices[index + u32(2)] = c;
        vertices[index + u32(3)] = b;
        vertices[index + u32(4)] = c;
        vertices[index + u32(5)] = d;
    } else {
        vertices[index] = a;
        vertices[index + u32(1)] = b;
        vertices[index + u32(2)] = d;
        vertices[index + u32(3)] = a;
        vertices[index + u32(4)] = c;
        vertices[index + u32(5)] = d;
    }
}

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