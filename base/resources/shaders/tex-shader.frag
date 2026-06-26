#version 450

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec2 texCoord;

layout(binding = 1) uniform sampler2D texSampler;
layout(binding = 2) uniform texture2D myTexture;   

layout(location = 0) out vec4 outColor;

void main() {
    vec4 texColor = texture(texSampler, texCoord);
    outColor = vec4(fragColor, 1.0) * texColor;
}