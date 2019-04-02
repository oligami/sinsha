# version 450
# extension GL_ARB_separate_shader_objects : enable

// input
layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec4 in_color;

// output
layout(location = 0) out vec4 out_color;

// Push Constants
layout(push_constant) uniform Camera {
    vec3 pos;
    float theta;
    float phi;
} camera;

// Descriptor Set
layout(binding = 0) uniform Obj {
    vec3 pos;
    float theta;
    float phi;
} obj;

// Specialization Constants:
// height / width (= 1.0 / aspect)
layout(constant_id = 0) const float ASPECT_RE = 0.0;

// 1.0 / tan(fov)
layout(constant_id = 1) const float TAN_FOV_RE = 0.0;

// near and far of camera
layout(constant_id = 2) const float NEAR = 0.0;
layout(constant_id = 3) const float FAR = 0.0;

void main() {
    // translate the obj
    vec3 pos = in_position + obj.pos - camera.pos;

    // TODO: rotate the obj by the state of it

    // rotate the obj by camera state
    // rotate the obj around y (camera theta)
    float sin_cam_theta = sin(camera.theta);
    float cos_cam_theta = cos(camera.theta);
    pos.x = pos.z * sin_cam_theta + pos.x * cos_cam_theta;
    pos.z = pos.z * cos_cam_theta - pos.x * sin_cam_theta;
    // rotate the obj around x (camera phi)
    float sin_cam_phi = sin(camera.phi);
    float cos_cam_phi = cos(camera.phi);
    pos.y = pos.z * cos_cam_phi + pos.y * sin_cam_phi;
    pos.z = - pos.z * sin_cam_phi + pos.y * cos_cam_phi;

    gl_Position = vec4(
        pos.x * TAN_FOV_RE * ASPECT_RE,
        pos.y * TAN_FOV_RE,
        FAR * (pos.z - NEAR) / (FAR - NEAR),
        pos.z
    );
    out_color = in_color;
}