#version 450

layout(set = 0, binding = 0) uniform sampler2D uTexture;

// Push constants: 128 bytes total.
//   offset  0: vec4 targetColors[4]  (64 bytes — up to 4 target colours)
//   offset 64: vec4 outputColor      (16 bytes — replacement colour)
//   offset 80: vec4 borderColor      (16 bytes — dynamic-border colour)
//   offset 96: float sensitivity     ( 4 bytes — colour-distance threshold)
//   offset100: int   colorCount      ( 4 bytes — active target colours, 0-4)
//   offset104: int   colorPassthrough( 4 bytes — 1 = keep original RGB)
//   offset108: int   borderWidth     ( 4 bytes — neighbour-sample radius; 0 = off)
//   offset112: vec2  screenPixel     ( 8 bytes — 1/outputW, 1/outputH)
//   offset120: int   gammaMode       ( 4 bytes — 0=Auto, 1=sRGB, 2=Linear)
//   offset124: pad to 128

layout(push_constant) uniform PushConstants {
    vec4  targetColors[4];
    vec4  outputColor;
    vec4  borderColor;
    float sensitivity;
    int   colorCount;
    int   colorPassthrough;
    int   borderWidth;
    vec2  screenPixel;
    int   gammaMode;
} pc;

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 FragColor;

vec3 SRGBToLinear(vec3 c) {
    bvec3 cutoff = lessThanEqual(c, vec3(0.04045));
    vec3 low = c / 12.92;
    vec3 high = pow((c + 0.055) / 1.055, vec3(2.4));
    return mix(high, low, vec3(cutoff));
}

bool matchesTarget(vec3 rgb) {
    vec3 rgbLinear = SRGBToLinear(rgb);

    for (int i = 0; i < pc.colorCount; i++) {
        vec3 targetSRGB = pc.targetColors[i].rgb;
        vec3 targetLinear = SRGBToLinear(targetSRGB);

        float dist;
        if (pc.gammaMode == 2) {
            dist = distance(rgb, targetLinear);
        } else if (pc.gammaMode == 1) {
            dist = distance(rgbLinear, targetLinear);
        } else {
            float distSRGB = distance(rgb, targetSRGB);
            float distLinear = distance(rgbLinear, targetLinear);
            dist = min(distSRGB, distLinear);
        }

        if (dist < pc.sensitivity) {
            return true;
        }
    }
    return false;
}

void main() {
    vec4 texel = texture(uTexture, fragTexCoord);

    // No target colours → raw passthrough (unfiltered mirror).
    if (pc.colorCount <= 0) {
        FragColor = texel;
        return;
    }

    // Current pixel matches a target colour.
    if (matchesTarget(texel.rgb)) {
        if (pc.colorPassthrough != 0) {
            FragColor = vec4(texel.rgb, 1.0);
        } else {
            FragColor = vec4(pc.outputColor.rgb, 1.0);
        }
        return;
    }

    // Dynamic border: iterate at output-pixel granularity, but snap each
    // neighbor sample to the nearest texel center. This replicates toolscreen's
    // two-pass approach (filter → upscale → border) in a single pass:
    //
    //   screenPixel = 1/outputW, 1/outputH  (output pixel step)
    //   borderWidth = 2                      (2 output pixels)
    //
    // For an 11×7 texture at 8× scale (88×56 output), stepping 2 output
    // pixels = 0.25 texels. Fragments near a texel boundary cross into the
    // neighbor texel; interior fragments stay on the same texel (no border).
    // Result: a crisp 2-output-pixel border at texel edges.
    if (pc.borderWidth > 0) {
        vec2 texSize = vec2(textureSize(uTexture, 0));

        for (int dx = -pc.borderWidth; dx <= pc.borderWidth; dx++) {
            for (int dy = -pc.borderWidth; dy <= pc.borderWidth; dy++) {
                if (dx == 0 && dy == 0) continue;
                vec2 offset = vec2(float(dx), float(dy)) * pc.screenPixel;
                vec2 neighborUV = fragTexCoord + offset;

                // Snap to the texel center that NEAREST filtering would pick.
                // This avoids sub-texel interpolation artefacts: we always
                // compare against an actual source texel colour.
                vec2 snapped = (floor(neighborUV * texSize) + 0.5) / texSize;

                vec3 neighbor = texture(uTexture, snapped).rgb;
                if (matchesTarget(neighbor)) {
                    FragColor = pc.borderColor;
                    return;
                }
            }
        }
    }

    discard;
}
