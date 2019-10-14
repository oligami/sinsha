# version 450

// vertices
// TRIANGLE FAN
vec2 VERTICES[4] = vec2[] (
    vec2 (-1.0, -1.0),
    vec2 (-1.0,  1.0),
    vec2 ( 1.0,  1.0),
    vec2 ( 1.0, -1.0)
);

// input
layout(location = 0) in vec2 in_tex;

// output
layout(location = 0) out vec2 tex_xy;

layout(set = 0, binding = 0) uniform Something {
    vec2 delta;
    vec2 scale;
} A;

void main() {
    VERTICES[gl_VertexIndex] * A.scale + A.delta;
    tex_xy = in_tex;
}