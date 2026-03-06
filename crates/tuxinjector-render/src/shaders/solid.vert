#version 450

void main() {
    vec2 pos = vec2(
        float((gl_VertexIndex & 1) * 4 - 1),
        float((gl_VertexIndex & 2) * 2 - 1)
    );
    // No Y-flip needed (solid color — no UV interpolation, scissor handles rect).
    gl_Position = vec4(pos, 0.0, 1.0);
}
