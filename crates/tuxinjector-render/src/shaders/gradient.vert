#version 450

layout(location = 0) out vec2 fragUV;

void main() {
    vec2 pos = vec2(
        float((gl_VertexIndex & 1) * 4 - 1),
        float((gl_VertexIndex & 2) * 2 - 1)
    );
    // pos.y=-1 → NDC y=-1 → Vulkan framebuffer row 0 = TOP.
    // No Y-flip needed: standard Vulkan positive-height viewport already maps
    // NDC y=-1 to screen top, so fragUV.y=0 lands at the top of the screen.
    gl_Position = vec4(pos, 0.0, 1.0);

    // Map clip-space [-1,1] to UV [0,1].
    fragUV = pos * 0.5 + 0.5;
}
