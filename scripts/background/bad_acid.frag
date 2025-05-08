#version 300 es
precision mediump float;

#ifndef saturate
#define saturate(v) clamp(v,0.,1.)
#endif


out vec4 fragColor;

in vec2 coord;
uniform float time;
uniform float x_motion;
uniform float y_motion;


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

void main() {
    vec2 st = gl_FragCoord.xy / gl_FragCoord.w;

    float x_offset = (noise(st / 1000.0 + 900.0 + time + sin(time)) - 0.5) * 100.0;
    float y_offset = (noise(st / 1000.0 + 100.0 + time + cos(time)) - 0.5) * 100.0;

    float R = (noise(st / 100000.0 + vec2(x_offset,y_offset) / 10.0 + 910.0 +    x_motion * 10.0));
    float G = (noise(st / 100000.0 + vec2(x_offset,y_offset) / 10.0 + 1310.0 +   y_motion * 10.0));
    float B = (noise(st / 100000.0 + vec2(x_offset,y_offset) / 10.0 + 100.0 +    mix(x_motion, y_motion, 0.5) * 10.0));

    fragColor = vec4(R,G,B, 1.0) + time * 0.0;
}