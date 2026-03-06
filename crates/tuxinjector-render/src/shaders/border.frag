#version 450

layout(push_constant) uniform PushConstants {
    vec4 color;
    vec4 rect;           // x, y, w, h in pixels
    float borderWidth;
    float radius;
    vec2 resolution;
} pc;

layout(location = 0) in vec2 fragCoord;

layout(location = 0) out vec4 FragColor;

// Signed distance function for a rounded rectangle centred at the origin.
// `halfSize` is the half-extent and `r` is the corner radius.
float sdRoundedRect(vec2 p, vec2 halfSize, float r) {
    vec2 q = abs(p) - halfSize + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

void main() {
    // Rectangle centre and half-size.
    vec2 centre = pc.rect.xy + pc.rect.zw * 0.5;
    vec2 halfSize = pc.rect.zw * 0.5;

    // Clamp radius so it doesn't exceed the half-size.
    float r = min(pc.radius, min(halfSize.x, halfSize.y));

    // SDF distance from the fragment to the rounded rectangle.
    float d = sdRoundedRect(fragCoord - centre, halfSize, r);

    // The border occupies the band where |d| < borderWidth/2.
    float halfBW = pc.borderWidth * 0.5;

    // outer_mask: 1 inside the outer edge (d < halfBW), fades to 0 outside.
    // inner_mask: 1 outside the inner edge (d > -halfBW), fades to 0 inside.
    // Their product is 1 only within the border band.
    float outer_mask = 1.0 - smoothstep(halfBW - 0.5, halfBW + 0.5, d);
    float inner_mask = smoothstep(-halfBW - 0.5, -halfBW + 0.5, d);
    float mask = outer_mask * inner_mask;

    if (mask <= 0.0) {
        discard;
    }

    FragColor = vec4(pc.color.rgb, pc.color.a * mask);
}
