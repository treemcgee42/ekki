#version 450

layout (location = 0) out vec4 outColor;

layout(push_constant) uniform Push {
   mat4 model_matrix;
   vec4 color;
} push;

void main() {
    outColor = push.color;
}