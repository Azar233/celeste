#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct WeatherMaterial {
    weather_data: vec4<f32>,
};

@group(2) @binding(0)
var<uniform> weather: WeatherMaterial;

const LAYERS: i32 = 8;
const DEPTH: f32 = 0.1;
const WIDTH: f32 = 0.8;
const SPEED: f32 = 1.5;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let time = weather.weather_data.x;
    let p = mat3x3<f32>(
        13.323122, 23.5112, 21.71123,
        21.1212, 28.7312, 11.9312,
        21.8112, 14.7212, 61.3934,
    );

    var acc = vec3<f32>(0.0);
    let dof = 5.0 * sin(time * 0.1);

    for (var i: i32 = 0; i < LAYERS; i = i + 1) {
        let fi = f32(i);
        var q = in.uv * (5.5 + fi * DEPTH);
        let width_offset = WIDTH * (fract(fi * 7.238917) - WIDTH * 0.5);
        q += vec2<f32>(SPEED * time / (1.0 + fi * DEPTH * 0.3), q.x * width_offset);

        let n = vec3<f32>(floor(q), 31.189 + fi);
        let m = floor(n) * 0.00001 + fract(n);
        let mp = (31415.9 + m) / fract(p * m);
        let r = fract(mp);

        var s = abs(fract(q) - 0.5 + 0.9 * r.xy - 0.45);
        s += 0.01 * abs(2.0 * fract(10.0 * q.yx));

        let d = 0.6 * max(s.x - s.y, s.x + s.y) + max(s.x, s.y) - 0.001;
        let edge = 0.005 + 0.05 * min(0.5 * abs(fi - 5.0 - dof), 1.0);
        let layer = smoothstep(edge, -edge, d) * (r.x / max(0.02 * fi * DEPTH, 0.001));
        acc += vec3<f32>(layer);
    }

    let intensity = clamp(acc.x, 0.0, 1.0);
    if intensity <= 0.0001 {
        return vec4<f32>(1.0, 1.0, 1.0, 0.0);
    }

    return vec4<f32>(vec3<f32>(intensity), intensity);
}