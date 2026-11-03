//! Evil Edge Rays: light streaks shooting out of the image's edges from a centre point.

use evil_common::{Px, luma, sample};

const MANIFEST: &str = r#"{
  "api": 1,
  "id": "com.evileffects.edge-rays",
  "name": "Evil Edge Rays",
  "category": "EVILEFFECTS",
  "version": "0.1.0",
  "author": "EVILEFFECTS contributors",
  "description": "Finds the edges and streaks them outward from a centre point as rays of light.",
  "params": [
    {"id": "center", "name": "Center", "type": "point", "default": [0.5, 0.5]},
    {"id": "threshold", "name": "Edge Threshold", "type": "slider", "default": 20, "min": 0, "max": 100, "decimals": 1},
    {"id": "length", "name": "Ray Length (%)", "type": "slider", "default": 40, "min": 0, "max": 100, "decimals": 1},
    {"id": "intensity", "name": "Intensity", "type": "slider", "default": 4, "min": 0, "max": 40, "sliderMax": 12, "decimals": 2},
    {"id": "color", "name": "Color", "type": "color", "default": [1.0, 0.85, 0.6, 1.0]},
    {"id": "source_color", "name": "Use Source Colors", "type": "checkbox", "default": true},
    {"id": "rays_only", "name": "Rays Only", "type": "checkbox", "default": false}
  ]
}"#;

evil_common::export_plugin!(MANIFEST, 11, render);

/// Samples per radial pass; three passes of 8 reach 8³ = 512 effective taps within the fuel budget.
const TAPS: usize = 8;
const PASSES: u32 = 3;

fn render(p: &[f64; 11], px: &mut [Px], w: usize, h: usize, _time: f64, _scale: f64) {
    let [
        cx,
        cy,
        threshold,
        length,
        intensity,
        cr,
        cg,
        cb,
        _,
        src_col,
        rays_only,
    ] = *p;
    let (cx, cy) = (cx as f32, cy as f32);
    let (t, len, k) = (
        threshold as f32 / 100.0,
        length as f32 / 100.0,
        intensity as f32,
    );
    // Edge mask: Sobel magnitude on straight luminance, soft over 0.1 above the threshold.
    let l: Vec<f32> = px
        .iter()
        .map(|c| if c[3] > 0.0 { luma(c) / c[3] } else { 0.0 })
        .collect();
    let at = |x: usize, y: usize| l[y.min(h - 1) * w + x.min(w - 1)];
    let mut rays: Vec<Px> = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let (xl, yu) = (x.saturating_sub(1), y.saturating_sub(1));
            let gx = at(x + 1, yu) + 2.0 * at(x + 1, y) + at(x + 1, y + 1)
                - at(xl, yu)
                - 2.0 * at(xl, y)
                - at(xl, y + 1);
            let gy = at(xl, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1)
                - at(xl, yu)
                - 2.0 * at(x, yu)
                - at(x + 1, yu);
            let m = ((gx.hypot(gy) / 4.0 - t) / 0.1).clamp(0.0, 1.0);
            let c = if src_col != 0.0 { px[i] } else { [1.0; 4] };
            c.map(|v| v * m)
        })
        .collect();
    // Radial blur toward the centre. Taps are spaced evenly in log scale so a pass's taps sit a
    // constant ratio apart, which the next, TAPS times finer pass fills exactly.
    if len > 0.0 {
        let total = (1.0 - len.min(0.99)).ln();
        let mut tmp = rays.clone();
        for pass in 0..PASSES {
            let span = total / (TAPS as f32).powi(pass as i32);
            // (scale, weight); the coarsest pass also fades the rays out along their length.
            let fade = if pass == 0 { 2.5 } else { 0.0 };
            let taps: Vec<(f32, f32)> = (0..TAPS)
                .map(|i| {
                    (
                        (span * i as f32 / TAPS as f32).exp(),
                        (-fade * i as f32 / TAPS as f32).exp(),
                    )
                })
                .collect();
            let norm: f32 = taps.iter().map(|t| t.1).sum();
            for y in 0..h {
                for x in 0..w {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let mut acc = [0.0f32; 4];
                    for &(s, wt) in &taps {
                        let v = sample(&rays, w, h, cx + (fx - cx) * s, cy + (fy - cy) * s);
                        (0..4).for_each(|j| acc[j] += v[j] * wt / norm);
                    }
                    tmp[y * w + x] = acc;
                }
            }
            std::mem::swap(&mut rays, &mut tmp);
        }
    }
    let tint = [cr as f32, cg as f32, cb as f32];
    for (c, r) in px.iter_mut().zip(&rays) {
        if rays_only != 0.0 {
            *c = [0.0; 4];
        }
        for j in 0..3 {
            c[j] += r[j] * k * tint[j];
        }
        c[3] += (1.0 - c[3]) * (r[3] * k).clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bright square in the middle of a black frame.
    fn square() -> Vec<Px> {
        (0..64 * 64)
            .map(|i| {
                if (24..40).contains(&(i % 64)) && (24..40).contains(&(i / 64)) {
                    [1.0; 4]
                } else {
                    [0.0, 0.0, 0.0, 1.0]
                }
            })
            .collect()
    }

    fn run(p: [f64; 11]) -> Vec<Px> {
        let mut px = square();
        render(&p, &mut px, 64, 64, 0.0, 1.0);
        px
    }

    #[test]
    fn zero_intensity_is_identity() {
        assert_eq!(
            run([32.0, 32.0, 20.0, 40.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0]),
            square()
        );
    }

    #[test]
    fn rays_reach_outward_past_the_edges_but_not_flat_areas() {
        let px = run([32.0, 32.0, 20.0, 60.0, 2.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0]);
        // Outside the square, along the axis away from the centre: lit by rays.
        assert!(px[32 * 64 + 50][0] > 0.01, "{:?}", px[32 * 64 + 50]);
        // Without length there are only the edges themselves; the flat inside stays dark.
        let edges = run([32.0, 32.0, 20.0, 0.0, 2.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0]);
        assert_eq!(edges[32 * 64 + 32][0], 0.0);
        assert!(edges[32 * 64 + 24][0] > 0.0);
    }
}
