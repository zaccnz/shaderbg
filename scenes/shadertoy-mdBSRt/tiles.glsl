struct RenderParams
{
    vec3 tile_colour;
    // because of how WGPU packs uniforms, we need a spacer here
    float _spacer; 
    float tile_speed;
};

layout(set = 0, binding = 1) uniform RenderParams params;

void mainImage( out vec4 fragColor, in vec2 fragCoord )
{
    float time = iTime * params.tile_speed;
    float aspect_ratio = iResolution.y/iResolution.x;
	vec2 uv = fragCoord.xy / iResolution.x;
    uv -= vec2(0.5, 0.5 * aspect_ratio);
    float rot = radians(-30. -time); // radians(45.0*sin(time));
    mat2 rotation_matrix = mat2(cos(rot), -sin(rot), sin(rot), cos(rot));
   	uv = rotation_matrix * uv;
    vec2 scaled_uv = 20.0 * uv; 
    vec2 tile = fract(scaled_uv);
    float tile_dist = min(min(tile.x, 1.0-tile.x), min(tile.y, 1.0-tile.y));
    float square_dist = length(floor(scaled_uv));
    
    float edge = sin(time-square_dist*20.);
    edge = mod(edge * edge, edge / edge);

    float value = mix(tile_dist, 1.0-tile_dist, step(1.0, edge));
    edge = pow(abs(1.0-edge), 2.2) * 0.5;
    
    value = smoothstep( edge-0.05, edge, 0.95*value);
    
    
    value += square_dist*.1;
    value *= 0.8 - 0.2;
    vec3 col = vec3(pow(value, 2.0)) * params.tile_colour;
    fragColor = vec4(col, 1.);
}