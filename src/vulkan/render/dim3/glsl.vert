# version 450

// input
layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec4 in_color;

// output
layout(location = 0) out vec3 out_position;
layout(location = 0) out vec3 out_normal;
layout(location = 0) out vec4 out_color;


// Camera uniform
layout(binding = 0, set = 0) uniform Camera {
    mat4 trans;
} Camera;

// Descriptor Set
layout(binding = 0, set = 1) uniform Obj {
    vec3 pos;
    float theta;
    float phi;
} obj;

// Specialization Constants:
layout(constant_id = 0) const mat4 PROJECTION = mat4(0);

void main() {
    // TODO: put the object in world coordinates.


    // out_position = ...
    // out_normal = ...
    out_color = in_color;
}