#version 450

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec2 texCoord;

layout(binding = 1) uniform sampler texSampler;
layout(binding = 2) uniform texture2D texImage;

layout(location = 0) out vec4 outColor;

void main() {
    vec4 texColor = texture(sampler2D(texImage, texSampler), texCoord);
    outColor = vec4(fragColor, 1.0) * texColor;
}