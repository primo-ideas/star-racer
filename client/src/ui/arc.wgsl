// One gauge of the HUD: an arc of neon drawn in its node, smooth and round-ended, with a soft
// glow around it, lit along part of its length (see gauges.rs). Every number is a vec4, which
// WebGL2's uniform rules want.

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct Arc {
    // Start and signed sweep of the arc, in radians clockwise from the right (the screen's y
    // points down); its radius and its line's thickness, in pixels.
    shape: vec4<f32>,
    // The share of the arc lit, from and to; how far the glow reaches, in pixels; how bright the
    // lit part burns, 1 being plain.
    fill: vec4<f32>,
    // The lit color, straight alpha.
    color: vec4<f32>,
    // The unlit line's color, straight alpha.
    dim: vec4<f32>,
    // How many pieces the arc is cut into (1 for a whole one); the gap between them, as a share of
    // the arc; where a notch crosses it, as a share (negative for none); how white the lit end
    // burns.
    cut: vec4<f32>,
};

@group(1) @binding(0) var<uniform> arc: Arc;

const PI: f32 = 3.14159265;

// Wraps an angle into [-π, π).
fn wrap(angle: f32) -> f32 {
    return angle - 2.0 * PI * floor((angle + PI) / (2.0 * PI));
}

// The point of the arc's circle at share `t` of the arc.
fn at(t: f32) -> vec2<f32> {
    let angle = arc.shape.x + arc.shape.y * t;
    return vec2<f32>(cos(angle), sin(angle)) * arc.shape.z;
}

// Distance from `p` to the piece of the arc from share `a` to share `b`, ends rounded, and the
// share of the arc `p` lies across from.
fn piece(p: vec2<f32>, a: f32, b: f32) -> vec2<f32> {
    let t = wrap(atan2(p.y, p.x) - arc.shape.x) / arc.shape.y;
    if (t >= a && t <= b) {
        return vec2<f32>(abs(length(p) - arc.shape.z), t);
    }
    let to_a = distance(p, at(a));
    let to_b = distance(p, at(b));
    if (to_a < to_b) {
        return vec2<f32>(to_a, a);
    }
    return vec2<f32>(to_b, b);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * in.size;
    let half = arc.shape.w * 0.5;
    let pieces = max(arc.cut.x, 1.0);
    let gap = arc.cut.y;

    // The nearest piece of the arc.
    var best = vec2<f32>(1e9, 0.0);
    for (var i = 0.0; i < pieces; i = i + 1.0) {
        let a = (i + select(0.0, gap * 0.5, i > 0.0)) / pieces;
        let b = (i + 1.0 - select(0.0, gap * 0.5, i < pieces - 1.0)) / pieces;
        let found = piece(p, a, b);
        if (found.x < best.x) {
            best = found;
        }
    }
    let d = best.x - half;
    let t = best.y;

    // Inside the line, with a pixel of soft edge, and a little rim of light just outside it.
    let inside = 1.0 - smoothstep(-0.75, 0.75, d);
    let lit = t >= arc.fill.x - 0.0001 && t <= arc.fill.y + 0.0001 && arc.fill.y > arc.fill.x;

    var rgb = vec3<f32>(0.0);
    var alpha = 0.0;
    if (lit) {
        // Hot in the middle of the line, the color toward its edges, a glow fading out around it.
        let across = clamp(1.0 - (best.x / max(half, 0.5)), 0.0, 1.0);
        let head = smoothstep(0.06, 0.0, abs(t - arc.fill.y)) * arc.cut.w;
        let white = clamp(across * across * 0.55 + head, 0.0, 1.0);
        let line = mix(arc.color.rgb * arc.fill.w, vec3<f32>(1.0), white);
        let glow = exp(-max(d, 0.0) / max(arc.fill.z, 0.5)) * 0.75 * (1.0 - inside);
        rgb = line * inside + arc.color.rgb * glow;
        alpha = max(inside * arc.color.a, glow * arc.color.a);
    } else {
        // Unlit: the line quiet, a faint rim outlining it.
        let rim = (1.0 - smoothstep(0.0, 2.0, abs(d))) * 0.6;
        rgb = arc.dim.rgb;
        alpha = max(inside * arc.dim.a, rim * arc.dim.a);
    }

    // The notch: a short white bar across the line.
    if (arc.cut.z >= 0.0) {
        let mark = at(arc.cut.z);
        let normal = normalize(mark);
        let tangent = vec2<f32>(-normal.y, normal.x);
        let along = abs(dot(p - mark, tangent));
        let out = abs(dot(p - mark, normal));
        let notch = (1.0 - smoothstep(0.8, 1.6, along)) * (1.0 - smoothstep(half + 4.0, half + 5.0, out));
        rgb = mix(rgb, vec3<f32>(1.0), notch);
        alpha = max(alpha, notch * 0.95);
    }
    return vec4<f32>(rgb, alpha);
}
