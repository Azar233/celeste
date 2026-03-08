#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> fill_color: vec4<f32>;
@group(2) @binding(1) var<uniform> outline_color: vec4<f32>;
@group(2) @binding(2) var<uniform> effect_params: vec4<f32>;

fn quantize_uv(uv: vec2<f32>, steps: f32) -> vec2<f32> {
    let grid = max(steps, 1.0);
    return (floor(uv * grid) + vec2<f32>(0.5, 0.5)) / grid;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_steps = effect_params.x;
    let outline_width = effect_params.y;
    let square_mix = effect_params.z;

    let pixel_uv = quantize_uv(mesh.uv, pixel_steps);
    let centered = pixel_uv * 2.0 - vec2<f32>(1.0, 1.0);

    let circle_distance = length(centered);
    let square_distance = max(abs(centered.x), abs(centered.y));
    let shape_distance = mix(circle_distance, square_distance, square_mix);

    let outer_mask = select(0.0, 1.0, shape_distance <= 1.0);
    if outer_mask == 0.0 {
        discard;
    }

    let inner_limit = max(0.0, 1.0 - outline_width);
    let inner_mask = select(0.0, 1.0, shape_distance <= inner_limit);
    let outline_mask = outer_mask - inner_mask;

    return outline_color * outline_mask + fill_color * inner_mask;
}