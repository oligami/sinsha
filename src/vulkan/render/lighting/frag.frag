#version 450

layout(location = 0) out vec4 out_color;
layout(set = 0, binding = 0, input_attachment_index = 0) uniform subpassInput POSITION;
layout(set = 0, binding = 0, input_attachment_index = 1) uniform subpassInput NORMAL;
layout(set = 0, binding = 0, input_attachment_index = 2) uniform subpassInput COLOR;

void main() {
    // Do some lighting.
    out_color = SubpassLoad(COLOR);
}
