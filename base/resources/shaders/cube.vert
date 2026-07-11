#version 450

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout(location = 0) in vec4 inPosition;
layout(location = 1) in vec4 inNormal;
layout(location = 2) in vec2 inUv;
layout(location = 3) in uvec4 inOffset;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 fragUv;


void main() {
    gl_Position = ubo.proj * ubo.view * ubo.model * vec4(inPosition.xyz, 1.0);
    fragColor = vec3(1.0);
    fragUv = inUv;
}