//! Evil Shake: hard camera shake with motion blur.

use evil_common::{MAX_SAMPLES, Px, noise, sample_edges};

const MANIFEST: &str = r#"{
  "api": 1,
  "id": "com.evileffects.shake",
  "name": "Evil Shake",
  "category": "EVILEFFECTS",
  "version": "0.1.0",
  "author": "EVILEFFECTS contributors",
  "description": "Random camera shake: position, rotation and zoom wobble with motion blur.",
  "params": [
    {"id": "amplitude", "name": "Amplitude (px)", "type": "slider", "default": 30, "min": 0, "max": 2000, "sliderMax": 200, "decimals": 1},
    {"id": "rotation", "name": "Rotation (deg)", "type": "slider", "default": 3, "min": 0, "max": 180, "sliderMax": 30, "decimals": 2},
    {"id": "zoom_shake", "name": "Zoom Shake (%)", "type": "slider", "default": 0, "min": 0, "max": 100, "sliderMax": 20, "decimals": 1},
    {"id": "frequency", "name": "Shakes per Second", "type": "slider", "default": 8, "min": 0, "max": 120, "sliderMax": 30, "decimals": 2},
    {"id": "roughness", "name": "Roughness", "type": "slider", "default": 30, "min": 0, "max": 100, "decimals": 1},
    {"id": "zoom", "name": "Zoom to Hide Edges (%)", "type": "slider", "default": 100, "min": 10, "max": 400, "sliderMin": 100, "sliderMax": 150, "decimals": 1},
    {"id": "blur", "name": "Motion Blur (shutter %)", "type": "slider", "default": 50, "min": 0, "max": 100, "decimals": 1},
    {"id": "samples", "name": "Blur Samples", "type": "slider", "default": 8, "min": 1, "max": 24, "decimals": 0},
    {"id": "edges", "name": "Edges", "type": "popup", "options": ["Transparent", "Repeat Edge", "Mirror"], "default": 2},
    {"id": "seed", "name": "Random Seed", "type": "slider", "default": 0, "min": 0, "max": 10000, "sliderMax": 100, "decimals": 0}
  ]
}"#;

evil_common::export_plugin!(MANIFEST, 10, render);

/// Shake offset at time `t`: (dx, dy, angle in radians, scale factor).
fn offset(p: &[f64; 10], t: f64, scale: f64) -> (f32, f32, f32, f32) {
    let [amp, rot, zshake, freq, rough, _, _, _, _, seed] = *p;
    let r = rough / 100.0;
    // Each axis gets its own seed; roughness mixes in a 4x faster octave.
    let n = |k: f64| {
        let s = seed * 7.0 + k * 101.0;
        (noise(t * freq, s) * (1.0 - r * 0.5) + noise(t * freq * 4.0, s + 50.0) * r * 0.5) as f32
    };
    let a = (amp * scale) as f32;
    (
        n(0.0) * a,
        n(1.0) * a,
        n(2.0) * (rot as f32).to_radians(),
        1.0 + n(3.0) * zshake as f32 / 100.0,
    )
}

fn render(p: &[f64; 10], px: &mut [Px], w: usize, h: usize, time: f64, scale: f64) {
    let (zoom, shutter, samples, edges) = (
        p[5] / 100.0,
        p[6] / 100.0,
        (p[7].round() as usize).clamp(1, MAX_SAMPLES),
        p[8],
    );
    let src = px.to_vec();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    // Sub-frame offsets across the shutter (one frame at 24 fps is 1/24 s).
    let n = if shutter > 0.0 { samples } else { 1 };
    let subs: Vec<_> = (0..n)
        .map(|i| {
            let t = time - shutter / 24.0 * i as f64 / n as f64;
            let (dx, dy, ang, z) = offset(p, t, scale);
            let s = 1.0 / (z * zoom as f32).max(0.01);
            (dx, dy, ang.cos() * s, ang.sin() * s)
        })
        .collect();
    let inv = 1.0 / n as f32;
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let mut acc = [0.0f32; 4];
            for &(dx, dy, c, s) in &subs {
                // Inverse transform: undo the shift, then rotate and scale back around the centre.
                let (ux, uy) = (fx - dx, fy - dy);
                let v = sample_edges(
                    &src,
                    w,
                    h,
                    cx + ux * c + uy * s,
                    cy - ux * s + uy * c,
                    edges,
                );
                (0..4).for_each(|k| acc[k] += v[k] * inv);
            }
            px[y * w + x] = acc;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> Vec<Px> {
        (0..64 * 32)
            .map(|i| {
                if (i % 64) / 8 % 2 == (i / 64) / 8 % 2 {
                    [1.0; 4]
                } else {
                    [0.0, 0.0, 0.0, 1.0]
                }
            })
            .collect()
    }

    #[test]
    fn no_shake_is_identity() {
        let mut px = card();
        render(
            &[0.0, 0.0, 0.0, 8.0, 30.0, 100.0, 50.0, 8.0, 2.0, 0.0],
            &mut px,
            64,
            32,
            1.3,
            1.0,
        );
        let diff = px
            .iter()
            .zip(card())
            .map(|(a, b)| (a[0] - b[0]).abs())
            .fold(0.0, f32::max);
        assert!(diff < 1e-4, "{diff}");
    }

    #[test]
    fn moves_over_time_and_is_deterministic() {
        let p = [20.0, 5.0, 5.0, 8.0, 30.0, 110.0, 50.0, 8.0, 2.0, 4.0];
        let at = |t| {
            let mut px = card();
            render(&p, &mut px, 64, 32, t, 1.0);
            px
        };
        assert_ne!(at(0.1), at(0.6));
        assert_eq!(at(0.6), at(0.6));
        // Mirror edges keep the frame fully opaque.
        assert!(at(0.6).iter().all(|c| (c[3] - 1.0).abs() < 1e-4));
    }
}
