// sdfhujg

struct Uniforms {
	matrix: mat3x2f,
	color: vec3f,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2f,
    @location(1) shade: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) color: vec3f,
};

@vertex
fn vs_main( model: VertexInput ) -> VertexOutput {
    return VertexOutput(vec4f(dot(model.position, uniforms.matrix[0]) + uniforms.matrix[2][0], dot(model.position, uniforms.matrix[1]) + uniforms.matrix[2][1], 0.0, 1.0), model.shade * uniforms.color);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4f(in.color, 1.0);
}