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
    {"id": "samples", "name": "Samples", "type": "slider", "default": 12, "min": 2, "max": 16, "decimals": 0},
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

/// Taps of the second, short pass that fills the gaps between the path's samples.
const FILL: usize = 8;

/// Averages `src` over the offsets into `out`.
fn smear(src: &[Px], out: &mut [Px], w: usize, h: usize, offsets: &[(f32, f32)], edges: f64) {
    let inv = 1.0 / offsets.len() as f32;
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 4];
            for &(dx, dy) in offsets {
                let v = sample_edges(src, w, h, x as f32 + 0.5 - dx, y as f32 + 0.5 - dy, edges);
                (0..4).for_each(|k| acc[k] += v[k] * inv);
            }
            out[y * w + x] = acc;
        }
    }
}

fn render(p: &[f64; 6], px: &mut [Px], w: usize, h: usize, _time: f64, scale: f64) {
    let length = (p[0] * scale) as f32;
    if length <= 0.0 {
        return;
    }
    let n = (p[4].round() as usize).clamp(2, MAX_SAMPLES - FILL);
    let pts = path(length, p[1] as f32, p[2] as f32, p[3] != 0.0, n);
    let src = px.to_vec();
    smear(&src, px, w, h, &pts, p[5]);
    // Blurs are separable, so a second pass of FILL taps spanning one step (along the mean heading)
    // turns n copies into a smooth smear: n × FILL effective samples within the fuel budget.
    let ds = length / (n - 1) as f32;
    if ds > 1.0 {
        let th = ((p[1] + p[2] / 2.0) as f32).to_radians();
        let fill: Vec<(f32, f32)> = (0..FILL)
            .map(|k| {
                let t = ((k as f32 + 0.5) / FILL as f32 - 0.5) * ds;
                (th.cos() * t, -th.sin() * t)
            })
            .collect();
        let coarse = px.to_vec();
        smear(&coarse, px, w, h, &fill, p[5]);
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

    #[test]
    fn long_blur_has_no_gaps_between_samples() {
        let (w, h) = (161, 11);
        let mut px = vec![[0.0; 4]; w * h];
        px[5 * w + 80] = [1.0; 4];
        // 100 px with 11 samples: 10 px steps, which alone would leave dark gaps.
        render(&[100.0, 0.0, 0.0, 1.0, 11.0, 0.0], &mut px, w, h, 0.0, 1.0);
        let row: Vec<f32> = (40..120).map(|x| px[5 * w + x][0]).collect();
        assert!(row.iter().all(|v| *v > 0.0), "{row:?}");
    }
}
