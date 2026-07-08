#version 450

layout(binding = 0) uniform UniformBufferObject {
    mat4 proj;
} ubo;

layout(location = 0) in uvec2 inPosition;
layout(location = 1) in uvec2 inSize;
layout(location = 2) in uvec4 inColor;
layout(location = 3) in uvec2 inTexCoord;
layout(location = 4) in uvec2 inTexSize;
layout(location = 5) in uint  inLayerIndex;

layout(location = 0) out vec4 fragColor;
layout(location = 1) out vec2 fragTexCoord;
layout(location = 2) flat out uint layerIndex;


void main() {
    vec2 pos = vec2(inPosition);
    vec2 size = vec2(inSize);
    vec4 color = vec4(inColor);
    vec2 uv = vec2(inTexCoord);
    vec2 uvSize = vec2(inTexSize);
    vec2 offset = vec2(0.0);
    vec2 outUv = vec2(0.0);

    if (gl_VertexIndex == 0) {
        offset = vec2(0.0, 0.0); 
        outUv = vec2(uv.x, uv.y);
    } else if (gl_VertexIndex == 1) {
        offset = vec2(0.0, size.y); 
        outUv = vec2(uv.x, uvSize.y);
    } else if (gl_VertexIndex == 2) {
        offset = vec2(size.x, 0.0); 
        outUv = vec2(uvSize.x, uv.y);
    } else if (gl_VertexIndex == 3) {
        offset = vec2(size.x, size.y); 
        outUv = vec2(uvSize.x, uvSize.y);
    }

    fragColor = inColor;
    fragTexCoord = outUv;
    layerIndex = inLayerIndex;
    gl_Position = ubo.proj * vec4(pos.xy + offset, 0.0, 1.0);
}