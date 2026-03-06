#version 450

layout(set = 0, binding = 0) uniform sampler2D uTexture;

layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 FragColor;

layout(push_constant) uniform PushData {
    int circleClip;   // 0 = normal rect, 1 = clip to circle
} push;

void main() {
    if (push.circleClip != 0) {
        // SDF circle clip in UV space: center at (0.5, 0.5), radius 0.5.
        // Fragments outside the inscribed circle are discarded entirely.
        vec2 uv = fragTexCoord - vec2(0.5);
        if (dot(uv, uv) > 0.25) {
            discard;
        }
    }
    FragColor = texture(uTexture, fragTexCoord);
}
