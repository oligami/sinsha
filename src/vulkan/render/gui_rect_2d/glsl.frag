# version 450

// INPUTS //
// 0. Texture Coordinates
layout(location = 0) in vec2 tex_in;

// OUTPUTS //
// 0. The Color to be Written in Framebuffer
layout(location = 0) out vec4 color_out;

// UNIFORM //
// 0-1. Texture Image of the Rect2D
layout(binding = 0, set = 1) uniform sampler2D texture_image;

void main() {
    color_out = texture(texture_image, tex_in);
}