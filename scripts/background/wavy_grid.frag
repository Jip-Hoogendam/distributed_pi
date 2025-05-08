#version 300 es
precision mediump float;
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

float plot(vec2 st) {
    float grid = min(abs(sin(st.x * 20.0)), abs(sin(st.y * 20.0)));
    return smoothstep(0.05,0.0,grid);
}


void main() {
    vec2 st = gl_FragCoord.xy / gl_FragCoord.w / 1000.0;

    st.x += noise(st + x_motion + 900.0) / 2.02;
    st.y += noise(st + y_motion + 100.0) / 2.02;
    
    float color_pct = noise(st * 1.0 + 100.0);

    vec3 color = (1.0-color_pct)*vec3(0.0)+color_pct*vec3(1.0,0.4,0.7);

    // Plot a line
    float pct = plot(st);
    color = (1.0-pct)*color+pct*vec3(1.0);

    fragColor = vec4(color, 1.0) + time * 0.0;
}