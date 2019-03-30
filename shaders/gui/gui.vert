# version 450
# extension GL_ARB_separate_shader_objects : enable

// MAYBE: Vertex position will be represent in 3D coordinate.
layout(location = 0) in vec4 vert_position_and_texture;

layout(push_constant) uniform GuiUniform {
	vec4 color_weight;
	vec2 position_bias;
} uni;

layout(location = 0) out vec2 out_texture;
layout(location = 1) out vec4 color_weight;

// Specialization Constants: represent height/width ratio
layout(constant_id = 0) const float RATIO = 1.0;

void main() {
	float x = vert_position_and_texture.x * RATIO;
	gl_Position = vec4(vec2(x, vert_position_and_texture.y) + uni.position_bias, 0.0, 1.0);
	out_texture = vert_position_and_texture.zw;
	color_weight = uni.color_weight;
}