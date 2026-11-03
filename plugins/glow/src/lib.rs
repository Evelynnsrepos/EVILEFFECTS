//! Evil Glow: blurs the parts brighter than a threshold and adds them back on top.

use evil_common::{Px, blur, luma};

const MANIFEST: &str = r#"{
  "api": 1,
  "id": "com.evileffects.glow",
  "name": "Evil Glow",
  "category": "EVILEFFECTS",
  "version": "0.1.0",
  "author": "EVILEFFECTS contributors",
  "description": "Intense glow from the bright parts of the image.",
  "params": [
    {"id": "threshold", "name": "Threshold", "type": "slider", "default": 60, "min": 0, "max": 100, "decimals": 1},
    {"id": "radius", "name": "Radius (px)", "type": "slider", "default": 30, "min": 0, "max": 1000, "sliderMax": 200, "decimals": 1},
    {"id": "intensity", "name": "Intensity", "type": "slider", "default": 1.5, "min": 0, "max": 20, "sliderMax": 5, "decimals": 2},
    {"id": "layers", "name": "Glow Layers", "type": "slider", "default": 3, "min": 1, "max": 6, "decimals": 0},
    {"id": "tint", "name": "Color", "type": "color", "default": [1.0, 1.0, 1.0, 1.0]}
  ]
}"#;

evil_common::export_plugin!(MANIFEST, 8, render);

fn render(p: &[f64; 8], px: &mut [Px], w: usize, h: usize, _time: f64, scale: f64) {
    let [threshold, radius, intensity, layers, tr, tg, tb, _ta] = p.map(|v| v as f32);
    if intensity == 0.0 || radius <= 0.0 {
        return;
    }
    let t = threshold / 100.0;
    let bright: Vec<Px> = px
        .iter()
        .map(|c| {
            // Threshold on straight (un-premultiplied) luminance, soft over 0.1.
            let y = if c[3] > 0.0 { luma(c) / c[3] } else { 0.0 };
            let k = ((y - t) / 0.1).clamp(0.0, 1.0);
            c.map(|v| v * k * k * (3.0 - 2.0 * k))
        })
        .collect();
    // Several blur sizes, each double the last, give the tight core plus wide haze.
    let layers = (layers.round() as usize).clamp(1, 6);
    let mut glow = vec![[0.0f32; 4]; px.len()];
    let mut r = radius * scale as f32 / (1 << (layers - 1)) as f32;
    for _ in 0..layers {
        let mut b = bright.clone();
        blur(&mut b, w, h, r);
        for (g, v) in glow.iter_mut().zip(&b) {
            (0..4).for_each(|k| g[k] += v[k] / layers as f32);
        }
        r *= 2.0;
    }
    let tint = [tr, tg, tb];
    for (c, g) in px.iter_mut().zip(&glow) {
        for k in 0..3 {
            c[k] += g[k] * intensity * tint[k];
        }
        c[3] += (1.0 - c[3]) * (g[3] * intensity).clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dot() -> Vec<Px> {
        let mut px = vec![[0.0, 0.0, 0.0, 1.0]; 41 * 41];
        px[20 * 41 + 20] = [1.0; 4];
        px
    }

    #[test]
    fn zero_intensity_is_identity() {
        let mut px = dot();
        render(
            &[60.0, 10.0, 0.0, 3.0, 1.0, 1.0, 1.0, 1.0],
            &mut px,
            41,
            41,
            0.0,
            1.0,
        );
        assert_eq!(px, dot());
    }

    #[test]
    fn bright_dot_glows_tinted_and_dark_stays_dark() {
        let mut px = dot();
        render(
            &[60.0, 12.0, 2.0, 3.0, 1.0, 0.0, 0.0, 1.0],
            &mut px,
            41,
            41,
            0.0,
            1.0,
        );
        let near = px[20 * 41 + 23];
        assert!(near[0] > 0.0 && near[1] == 0.0, "{near:?}");
        let mut dark = vec![[0.3, 0.3, 0.3, 1.0]; 41 * 41];
        render(
            &[60.0, 12.0, 2.0, 3.0, 1.0, 1.0, 1.0, 1.0],
            &mut dark,
            41,
            41,
            0.0,
            1.0,
        );
        assert!(dark.iter().all(|c| c[0] == 0.3));
    }
}
