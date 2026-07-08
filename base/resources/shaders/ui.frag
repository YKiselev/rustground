#version 450

layout(location = 0) in vec4 fragColor;
layout(location = 1) in vec2 texCoord;
layout(location = 2) flat in uint layerIndex;

layout(binding = 1) uniform sampler2DArray texSampler;
layout(binding = 2) uniform texture2D texImage;

layout(location = 0) out vec4 outColor;

void main() {
    float alpha = texture(texSampler, vec3(texCoord, float(layerIndex))).r;
    outColor = fragColor * vec4(1.0, 1.0, 1.0, alpha);
}