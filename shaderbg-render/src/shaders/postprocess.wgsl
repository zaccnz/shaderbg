struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var x = 0.0;
    var y = 0.0;
    var tex_coords = vec2<f32>(0.0, 1.0);
    if (in_vertex_index == u32(0)) {
        x = -1.0; y = -1.0;
    } else if (in_vertex_index == u32(1) || in_vertex_index == u32(3)) {
        x = -1.0; y =  1.0;
        tex_coords = vec2<f32>(0.0, 0.0);
    } else if (in_vertex_index == u32(2) || in_vertex_index == u32(5)) {
        x =  1.0; y = -1.0;
        tex_coords = vec2<f32>(1.0, 1.0);
    } else if (in_vertex_index == u32(4)) {
        x =  1.0; y =  1.0;
        tex_coords = vec2<f32>(1.0, 0.0);
    }

    var out: VertexOutput;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = tex_coords;
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@fragment
fn postprocess_to_srgb(in: VertexOutput) -> @location(0) vec4<f32> {
    let colour = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let srgb_colour = pow(colour, vec4<f32>(1.8));
    return srgb_colour;
}