//! Evil Motion Curves: motion blur along a bending path.

use evil_common::{MAX_SAMPLES, Px, sample_edges};

const MANIFEST: &str = r#"{
  "api": 1,
  "id": "com.evileffects.motion-curves",
  "name": "Evil Motion Curves",
  "category": "EVILEFFECTS",
  "version": "0.1.0",
  "author": "EVILEFFECTS contributors",
  "description": "Motion blur that smears the image along a curved path.",
  "params": [
    {"id": "length", "name": "Length (px)", "type": "slider", "default": 60, "min": 0, "max": 5000, "sliderMax": 400, "decimals": 1},
    {"id": "angle", "name": "Direction", "type": "angle", "default": 0},
    {"id": "curve", "name": "Curvature (deg)", "type": "slider", "default": 45, "min": -720, "max": 720, "sliderMin": -180, "sliderMax": 180, "decimals": 1},
    {"id": "centered", "name": "Centered", "type": "checkbox", "default": true},
    {"id": "samples", "name": "Samples", "type": "slider", "default": 16, "min": 2, "max": 24, "decimals": 0},
    {"id": "edges", "name": "Edges", "type": "popup", "options": ["Transparent", "Repeat Edge", "Mirror"], "default": 1}
  ]
}"#;

evil_common::export_plugin!(MANIFEST, 6, render);

/// Points along an arc of `length` that starts heading at `angle` and turns by `curve` degrees.
fn path(length: f32, angle: f32, curve: f32, centered: bool, n: usize) -> Vec<(f32, f32)> {
    let ds = length / (n - 1) as f32;
    let (mut x, mut y) = (0.0f32, 0.0f32);
    let mut pts = vec![(0.0, 0.0)];
    for i in 1..n {
        let th = (angle + curve * (i as f32 - 0.5) / (n - 1) as f32).to_radians();
        x += th.cos() * ds;
        y -= th.sin() * ds; // angles count anticlockwise, screen y points down
        pts.push((x, y));
    }
    if centered {
        let (mx, my) = pts[n / 2];
        pts.iter_mut().for_each(|p| *p = (p.0 - mx, p.1 - my));
    }
    pts
}

fn render(p: &[f64; 6], px: &mut [Px], w: usize, h: usize, _time: f64, scale: f64) {
    let length = (p[0] * scale) as f32;
    if length <= 0.0 {
        return;
    }
    let n = (p[4].round() as usize).clamp(2, MAX_SAMPLES);
    let pts = path(length, p[1] as f32, p[2] as f32, p[3] != 0.0, n);
    let src = px.to_vec();
    let inv = 1.0 / n as f32;
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 4];
            for &(dx, dy) in &pts {
                let v = sample_edges(&src, w, h, x as f32 + 0.5 - dx, y as f32 + 0.5 - dy, p[5]);
                (0..4).for_each(|k| acc[k] += v[k] * inv);
            }
            px[y * w + x] = acc;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_path_has_the_requested_length() {
        let pts = path(100.0, 0.0, 0.0, false, 11);
        let (x, y) = pts[10];
        assert!((x - 100.0).abs() < 1e-3 && y.abs() < 1e-3);
    }

    #[test]
    fn curvature_bends_the_path() {
        let (_, y) = path(100.0, 0.0, 90.0, false, 33)[32];
        assert!(y < -20.0, "{y}"); // turning anticlockwise goes up the screen
    }

    #[test]
    fn horizontal_blur_smears_a_dot_sideways_only() {
        let (w, h) = (41, 21);
        let mut px = vec![[0.0; 4]; w * h];
        px[10 * w + 20] = [1.0; 4];
        render(&[10.0, 0.0, 0.0, 1.0, 11.0, 1.0], &mut px, w, h, 0.0, 1.0);
        assert!(px[10 * w + 24][0] > 0.0 && px[10 * w + 16][0] > 0.0);
        assert_eq!(px[8 * w + 20][0], 0.0);
        let total: f32 = px.iter().map(|c| c[0]).sum();
        assert!((total - 1.0).abs() < 1e-3, "{total}");
    }
}
