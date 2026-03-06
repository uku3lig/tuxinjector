#version 450

layout(location = 0) out vec2 fragCoord;

layout(push_constant) uniform PushConstants {
    vec4 color;
    vec4 rect;           // x, y, w, h in pixels
    float borderWidth;
    float radius;
    vec2 resolution;
} pc;

void main() {
    vec2 pos = vec2(
        float((gl_VertexIndex & 1) * 4 - 1),
        float((gl_VertexIndex & 2) * 2 - 1)
    );
    // No Y-flip: Vulkan NDC y=-1 = framebuffer row 0 = screen top.
    gl_Position = vec4(pos, 0.0, 1.0);

    // Map clip-space to pixel coordinates.
    fragCoord = (pos * 0.5 + 0.5) * pc.resolution;
}
