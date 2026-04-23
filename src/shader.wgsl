// 유니폼 데이터 구조 (CPU에서 보내줄 데이터)
struct CameraUniform {
    offset: vec2<f32>,
};

@group(0) @binding(0) // 0번 그룹의 0번 바인딩으로 데이터를 받음
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) instance_pos: vec2<f32>, // 인스턴스 버퍼에서 오는 데이터
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // 원래 모양 좌표에 인스턴스 개별 위치를 더함
    out.clip_position = vec4<f32>(model.position.xy + model.instance_pos, model.position.z, 1.0);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}