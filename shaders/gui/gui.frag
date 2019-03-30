# version 450
# extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec2 in_texture;
layout(location = 1) in vec4 color_weight;

layout(binding = 0) uniform sampler2D texture_image;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = texture(texture_image, in_texture) * color_weight;
}