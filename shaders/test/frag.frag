# version 450
# extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 rgb_in;

layout(location = 0) out vec4 rgba_out;

void main() {
    rgba_out = vec4(rgb_in, 1.0);
}