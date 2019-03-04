# version 450
# extension GL_ARB_separate_shader_objects : enable

// MAYBE: Vertex position will be represent in 3D coordinate.
layout(location = 0) in vec4 vert_color;
layout(location = 1) in vec4 vert_position_and_texture;

layout(push_constant) uniform GuiUniform {
	vec4 color_weight;
	vec2 position_bias;
} uni;

layout(location = 0) out vec4 out_color;
layout(location = 1) out vec2 out_texture;

void main() {
	gl_Position = vec4(vert_position_and_texture.xy + uni.position_bias, 0.0, 1.0);
	out_color = vert_color * uni.color_weight;
	out_texture = vert_position_and_texture.wz;
}