//! Evil Warp Transform: move, scale and rotate with a wavy warp and streaky transform blur.

use evil_common::{MAX_SAMPLES, Px, sample_edges};

const MANIFEST: &str = r#"{
  "api": 1,
  "id": "com.evileffects.warp-transform",
  "name": "Evil Warp Transform",
  "category": "EVILEFFECTS",
  "version": "0.1.0",
  "author": "EVILEFFECTS contributors",
  "description": "Transform with a wavy warp and streaks from the untransformed image to the transformed one.",
  "params": [
    {"id": "center", "name": "Center", "type": "point", "default": [0.5, 0.5]},
    {"id": "shift_x", "name": "Shift X (px)", "type": "slider", "default": 0, "min": -10000, "max": 10000, "sliderMin": -500, "sliderMax": 500, "decimals": 1},
    {"id": "shift_y", "name": "Shift Y (px)", "type": "slider", "default": 0, "min": -10000, "max": 10000, "sliderMin": -500, "sliderMax": 500, "decimals": 1},
    {"id": "scale", "name": "Scale (%)", "type": "slider", "default": 100, "min": 1, "max": 1000, "sliderMax": 300, "decimals": 1},
    {"id": "angle", "name": "Rotation", "type": "angle", "default": 0},
    {"id": "warp", "name": "Warp Amount (px)", "type": "slider", "default": 0, "min": 0, "max": 1000, "sliderMax": 100, "decimals": 1},
    {"id": "warp_size", "name": "Warp Size (px)", "type": "slider", "default": 200, "min": 1, "max": 5000, "sliderMax": 1000, "decimals": 1},
    {"id": "warp_speed", "name": "Warp Speed", "type": "slider", "default": 1, "min": -50, "max": 50, "sliderMin": -5, "sliderMax": 5, "decimals": 2},
    {"id": "streaks", "name": "Streaks (%)", "type": "slider", "default": 0, "min": 0, "max": 100, "decimals": 1},
    {"id": "samples", "name": "Streak Samples", "type": "slider", "default": 16, "min": 2, "max": 24, "decimals": 0},
    {"id": "edges", "name": "Edges", "type": "popup", "options": ["Transparent", "Repeat Edge", "Mirror"], "default": 0}
  ]
}"#;

evil_common::export_plugin!(MANIFEST, 12, render);

fn render(p: &[f64; 12], px: &mut [Px], w: usize, h: usize, time: f64, scale: f64) {
    let [
        cx,
        cy,
        sx,
        sy,
        sc,
        ang,
        warp,
        size,
        speed,
        streaks,
        samples,
        edges,
    ] = *p;
    let (cx, cy) = (cx as f32, cy as f32);
    let k = scale as f32;
    let (sx, sy, warp, size) = (
        sx as f32 * k,
        sy as f32 * k,
        warp as f32 * k,
        (size as f32 * k).max(1.0),
    );
    let phase = (time * speed) as f32 * std::f32::consts::TAU;
    let src = px.to_vec();
    // Streaks blend from the untransformed image (u = 1 - streaks) to the full transform (u = 1).
    let n = if streaks > 0.0 {
        (samples.round() as usize).clamp(2, MAX_SAMPLES)
    } else {
        1
    };
    let steps: Vec<_> = (0..n)
        .map(|i| {
            let u = if n == 1 {
                1.0
            } else {
                1.0 - (streaks / 100.0) as f32 * i as f32 / (n - 1) as f32
            };
            let s = 1.0 / (1.0 + (sc as f32 / 100.0 - 1.0) * u).max(0.001);
            let a = (ang as f32).to_radians() * u;
            (sx * u, sy * u, a.cos() * s, a.sin() * s, warp * u)
        })
        .collect();
    let inv = 1.0 / n as f32;
    let tau = std::f32::consts::TAU / size;
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            // Sine warp: each axis is displaced by a wave travelling along the other (once per pixel; sin is costly in WebAssembly).
            let (wx, wy) = if warp > 0.0 {
                ((fy * tau + phase).sin(), (fx * tau + phase * 1.3).sin())
            } else {
                (0.0, 0.0)
            };
            let mut acc = [0.0f32; 4];
            for &(dx, dy, c, s, wa) in &steps {
                let (ux, uy) = (fx - dx - cx, fy - dy - cy);
                let (qx, qy) = (
                    cx + ux * c + uy * s + wa * wx,
                    cy - ux * s + uy * c + wa * wy,
                );
                let v = sample_edges(&src, w, h, qx, qy, edges);
                (0..4).for_each(|i| acc[i] += v[i] * inv);
            }
            px[y * w + x] = acc;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> Vec<Px> {
        (0..48 * 32)
            .map(|i| {
                if (i % 48) / 6 % 2 == 0 {
                    [1.0; 4]
                } else {
                    [0.0, 0.0, 0.0, 1.0]
                }
            })
            .collect()
    }

    fn run(p: [f64; 12]) -> Vec<Px> {
        let mut px = card();
        render(&p, &mut px, 48, 32, 0.5, 1.0);
        px
    }

    #[test]
    fn defaults_are_identity() {
        let diff = run([
            24.0, 16.0, 0.0, 0.0, 100.0, 0.0, 0.0, 200.0, 1.0, 0.0, 16.0, 0.0,
        ])
        .iter()
        .zip(card())
        .map(|(a, b)| (a[0] - b[0]).abs())
        .fold(0.0, f32::max);
        assert!(diff < 1e-4, "{diff}");
    }

    #[test]
    fn shift_moves_the_image() {
        let px = run([
            24.0, 16.0, 6.0, 0.0, 100.0, 0.0, 0.0, 200.0, 1.0, 0.0, 16.0, 0.0,
        ]);
        assert_eq!(px[10 * 48 + 20], card()[10 * 48 + 14]);
        // Pulled in from outside the left edge: transparent.
        assert_eq!(px[10 * 48][3], 0.0);
    }

    #[test]
    fn streaks_and_warp_change_the_image() {
        assert_ne!(
            run([
                24.0, 16.0, 6.0, 0.0, 150.0, 20.0, 0.0, 200.0, 1.0, 80.0, 16.0, 1.0
            ]),
            card()
        );
        assert_ne!(
            run([
                24.0, 16.0, 0.0, 0.0, 100.0, 0.0, 5.0, 20.0, 1.0, 0.0, 16.0, 1.0
            ]),
            card()
        );
    }
}
