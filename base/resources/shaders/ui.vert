#version 450

layout(binding = 0) uniform UniformBufferObject {
    mat4 proj;
} ubo;

layout(location = 0) in ivec2 inPosition;
layout(location = 1) in uvec2 inSize;
layout(location = 2) in vec4 inColor;
layout(location = 3) in vec2 inUvMin;
layout(location = 4) in vec2 inUvMax;
layout(location = 5) in uint  inLayerIndex;

layout(location = 0) out vec4 fragColor;
layout(location = 1) out vec2 fragTexCoord;
layout(location = 2) flat out uint layerIndex;


void main() {
    vec2 pos = vec2(inPosition);
    vec2 size = vec2(inSize);
    vec2 uvMin = inUvMin;
    vec2 uvMax = inUvMax;
    vec2 outUv = vec2(0.0);

    vec2 offset = vec2(
        float(gl_VertexIndex & 1),
        float((gl_VertexIndex >> 1) & 1)
    );

    if (gl_VertexIndex == 0) {
        outUv = vec2(uvMin.x, uvMin.y);
    } else if (gl_VertexIndex == 1) {
        outUv = vec2(uvMax.x, uvMin.y);
    } else if (gl_VertexIndex == 2) {
        outUv = vec2(uvMin.x, uvMax.y);
    } else if (gl_VertexIndex == 3) {
        outUv = vec2(uvMax.x, uvMax.y);
    }

    fragColor = inColor;
    fragTexCoord = outUv;
    layerIndex = inLayerIndex;
    gl_Position = ubo.proj * vec4(pos + offset * size, 0.0, 1.0);
}