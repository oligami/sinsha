# version 450

// input
layout(location = 0) in vec4 in_pos_and_tex;

// output
layout(location = 0) out vec2 tex_xy;

layout(push_constant) uniform something {
    vec2 origin;
    vec2 delta;
    float scale;
} SOme;

void main() {


    tex_xy = in_pos_and_tex.zw;
}