//! Evil Chroma Split: red pushed outward and blue pulled inward from a centre point.

use evil_common::{Px, sample};

const MANIFEST: &str = r#"{
  "api": 1,
  "id": "com.evileffects.chroma-split",
  "name": "Evil Chroma Split",
  "category": "EVILEFFECTS",
  "version": "0.1.0",
  "author": "EVILEFFECTS contributors",
  "description": "Splits red and blue apart radially from a centre point.",
  "params": [
    {"id": "amount", "name": "Shift at Corners (px)", "type": "slider", "default": 8, "min": -500, "max": 500, "sliderMin": -50, "sliderMax": 50, "decimals": 1},
    {"id": "center", "name": "Center", "type": "point", "default": [0.5, 0.5]},
    {"id": "falloff", "name": "Edge Falloff", "type": "slider", "default": 1, "min": 0, "max": 4, "decimals": 2}
  ]
}"#;

evil_common::export_plugin!(MANIFEST, 4, render);

fn render(p: &[f64; 4], px: &mut [Px], w: usize, h: usize, _time: f64, scale: f64) {
    let [amount, cx, cy, falloff] = p.map(|v| v as f32);
    let amount = amount * scale as f32;
    if amount == 0.0 {
        return;
    }
    let src = px.to_vec();
    // Normalize so the farthest corner from the centre gets the full shift.
    let far = [
        (0.0, 0.0),
        (w as f32, 0.0),
        (0.0, h as f32),
        (w as f32, h as f32),
    ]
    .iter()
    .map(|(x, y)| (x - cx).hypot(y - cy))
    .fold(1.0f32, f32::max);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let (dx, dy) = (fx - cx, fy - cy);
            let d = (dx * dx + dy * dy).sqrt();
            if d == 0.0 {
                continue;
            }
            let shift = amount * (d / far).powf(falloff) / d;
            let r = sample(&src, w, h, fx - dx * shift, fy - dy * shift);
            let g = src[y * w + x];
            let b = sample(&src, w, h, fx + dx * shift, fy + dy * shift);
            // Alpha covers all three channels so premultiplied values stay valid.
            px[y * w + x] = [r[0], g[1], b[2], r[3].max(g[3]).max(b[3])];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(w: usize, h: usize) -> Vec<Px> {
        (0..w * h)
            .map(|i| {
                if (i % w) / 4 % 2 == 0 {
                    [1.0; 4]
                } else {
                    [0.0, 0.0, 0.0, 1.0]
                }
            })
            .collect()
    }

    #[test]
    fn zero_amount_is_identity() {
        let mut px = card(32, 16);
        render(&[0.0, 16.0, 8.0, 1.0], &mut px, 32, 16, 0.0, 1.0);
        assert_eq!(px, card(32, 16));
    }

    #[test]
    fn splits_channels_away_from_centre_and_stays_premultiplied() {
        let mut px = card(32, 16);
        render(&[6.0, 16.0, 8.0, 1.0], &mut px, 32, 16, 0.0, 1.0);
        assert!(px.iter().any(|c| (c[0] - c[2]).abs() > 0.2));
        assert!(px.iter().all(|c| c[..3].iter().all(|v| *v <= c[3] + 1e-6)));
        // The centre pixel's neighbourhood barely moves.
        assert!((px[8 * 32 + 16][0] - card(32, 16)[8 * 32 + 16][0]).abs() < 0.3);
    }
}
