#version 300 es
precision mediump float;
out vec4 fragColor;

in vec2 coord;
uniform float time;
uniform float x_motion;
uniform float y_motion;
uniform vec2 u_resolution;

// 2D Random
float random (in vec2 st) {
    return fract(sin(dot(st.xy,
                        vec2(12.9898,78.233)))
                * 43758.5453123);
}

// 2D Noise based on Morgan McGuire @morgan3d
// https://www.shadertoy.com/view/4dS3Wd
float noise (in vec2 st) {
    vec2 i = floor(st);
    vec2 f = fract(st);

    // Four corners in 2D of a tile
    float a = random(i);
    float b = random(i + vec2(1.0, 0.0));
    float c = random(i + vec2(0.0, 1.0));
    float d = random(i + vec2(1.0, 1.0));

    // Smooth Interpolation

    // Cubic Hermine Curve.  Same as SmoothStep()
    vec2 u = f*f*(3.0-2.0*f);
    // u = smoothstep(0.,1.,f);

    // Mix 4 coorners percentages
    return mix(a, b, u.x) +
            (c - a)* u.y * (1.0 - u.x) +
            (d - b) * u.x * u.y;
}

#define GRID_SIZE 50.0

float plot(vec2 st) {
    float grid = min(abs(sin(st.x * GRID_SIZE)), abs(sin(st.y * GRID_SIZE)));
    return smoothstep(0.05,0.0,grid) * 10.0;
}


void main() {
    vec2 st = gl_FragCoord.xy /  u_resolution;
    
    float color_pct = noise(st * 1.0 + 100.0) / 10000.0;

    vec3 color = (1.0-color_pct)*vec3(0.0)+color_pct*vec3(1.0,0.4,0.7);

    float falloff = distance(st,vec2(0.5)) / 10.0;
    falloff *= falloff;

    
    vec2 random_offset = vec2(random(vec2(time * 100.0 + st.y, 0.0)), random(vec2(time * 100.0 + st.x, 1000.0))) * falloff * 1.0;
    
    // grid
    vec3 r_pct = plot(st + random_offset + vec2(0.0,0.0)) * vec3(1.0,0.0,0.0)            / (falloff * 3000.0);
    vec3 g_pct = plot(st + random_offset + vec2(-falloff,-falloff)) * vec3(0.0,1.0,0.0)  / (falloff * 3000.0);
    vec3 b_pct = plot(st + random_offset + vec2(falloff,falloff)) * vec3(0.0,0.0,1.0)    / (falloff * 3000.0);
    vec3 grid_color = r_pct + g_pct + b_pct;
    



    color += grid_color;
    color = min(color, vec3(1.0));

    fragColor = vec4(color, 1.0) + time * 0.0;
}