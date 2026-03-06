#version 450

layout(set = 0, binding = 0) uniform sampler2D uTexture;

layout(push_constant) uniform PushConstants {
    float opacity;
    int enableColorKey;
    vec4 colorKey;
    float colorKeySensitivity;
} pc;

layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 FragColor;

void main() {
    vec4 texel = texture(uTexture, fragTexCoord);

    // Color key: discard fragment if the texel color is close to the key.
    if (pc.enableColorKey != 0) {
        float dist = distance(texel.rgb, pc.colorKey.rgb);
        if (dist < pc.colorKeySensitivity) {
            discard;
        }
    }

    // Multiply alpha by opacity.
    texel.a *= pc.opacity;

    FragColor = texel;
}
