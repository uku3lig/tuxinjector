#version 450

layout(push_constant) uniform PushConstants {
    vec4 color1;
    vec4 color2;
    float angle;         // radians
    float time;          // seconds
    int animationType;   // 0=none, 1=rotate, 2=slide, 3=wave, 4=spiral, 5=fade
} pc;

layout(location = 0) in vec2 fragUV;

layout(location = 0) out vec4 FragColor;

void main() {
    vec2 uv = fragUV;

    // Centre-relative coordinates for rotation/spiral effects.
    vec2 centred = uv - 0.5;

    float t = 0.0;

    switch (pc.animationType) {
        case 0: {
            // Static gradient along the configured angle.
            float cosA = cos(pc.angle);
            float sinA = sin(pc.angle);
            t = dot(uv, vec2(cosA, sinA));
            break;
        }
        case 1: {
            // Rotate: the gradient angle rotates over time.
            float a = pc.angle + pc.time;
            float cosA = cos(a);
            float sinA = sin(a);
            t = dot(uv, vec2(cosA, sinA));
            break;
        }
        case 2: {
            // Slide: the gradient slides along its axis over time.
            float cosA = cos(pc.angle);
            float sinA = sin(pc.angle);
            t = dot(uv, vec2(cosA, sinA)) + pc.time * 0.2;
            t = fract(t);
            break;
        }
        case 3: {
            // Wave: sinusoidal distortion across the gradient.
            float cosA = cos(pc.angle);
            float sinA = sin(pc.angle);
            float base = dot(uv, vec2(cosA, sinA));
            // Perpendicular axis for the wave.
            float perp = dot(uv, vec2(-sinA, cosA));
            t = base + 0.05 * sin(perp * 12.0 + pc.time * 3.0);
            break;
        }
        case 4: {
            // Spiral: radial + angular gradient that rotates over time.
            float r = length(centred) * 2.0;
            float a = atan(centred.y, centred.x);
            t = fract(r + a / 6.28318 + pc.time * 0.3);
            break;
        }
        case 5: {
            // Fade: oscillate blend factor between the two colours.
            float cosA = cos(pc.angle);
            float sinA = sin(pc.angle);
            float base = dot(uv, vec2(cosA, sinA));
            float fade = sin(pc.time) * 0.5 + 0.5;
            t = mix(base, fade, 0.5);
            break;
        }
        default: {
            float cosA = cos(pc.angle);
            float sinA = sin(pc.angle);
            t = dot(uv, vec2(cosA, sinA));
            break;
        }
    }

    t = clamp(t, 0.0, 1.0);
    FragColor = mix(pc.color1, pc.color2, t);
}
