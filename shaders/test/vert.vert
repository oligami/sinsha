# version 450
# extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 xyz_in;
layout(location = 1) in vec3 rgb_in;

layout(location = 0) out vec3 rgb_out;

void main() {
    gl_Position = vec4(xyz_in, 1.0);
    rgb_out = rgb_in;
}