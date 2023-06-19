// prepare ShaderToy uniforms
in vec2 texCoord;
layout(location = 0) out vec4 fragColor;

struct ShaderToy 
{
    vec4 resolution;
    float time;
    float time_delta;
    vec4 mouse;
};

layout(set = 0, binding = 0) uniform ShaderToy shadertoy;

vec3 iResolution = shadertoy.resolution.xyz;
float iTime = shadertoy.time;
float iTimeDelta = shadertoy.time_delta;
vec4 iMouse = shadertoy.mouse;

vec2 fragCoord = vec2(gl_FragCoord.x, iResolution.y - gl_FragCoord.y);

// insert ShaderToy code here
{{SOURCE}}

// call ShaderToy main
void main()
{
    mainImage(fragColor, fragCoord);
}