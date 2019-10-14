# version 450

// input
layout(location = 0) in vec2 tex_xy;

// output
layout(location = 0) out vec4 out_color;

// Descriptor Set
layout(binding = 0, set = 1) uniform sampler2D texture_image;

void main() {
    out_color = texture(texture_image, tex_xy);
}