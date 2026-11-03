//! Evil Lens Flare: a glowing light source with starburst, anamorphic streak, halo and ghosts.

use evil_common::{Px, hash};

const MANIFEST: &str = r#"{
  "api": 1,
  "id": "com.evileffects.lens-flare",
  "name": "Evil Lens Flare",
  "category": "EVILEFFECTS",
  "version": "0.1.0",
  "author": "EVILEFFECTS contributors",
  "description": "Lens flare with a core glow, starburst rays, anamorphic streak, halo ring and ghosts along the flare axis.",
  "params": [
    {"id": "light", "name": "Light Position", "type": "point", "default": [0.3, 0.3]},
    {"id": "brightness", "name": "Brightness (%)", "type": "slider", "default": 100, "min": 0, "max": 1000, "sliderMax": 300, "decimals": 1},
    {"id": "size", "name": "Size (%)", "type": "slider", "default": 100, "min": 1, "max": 1000, "sliderMax": 300, "decimals": 1},
    {"id": "color", "name": "Color", "type": "color", "default": [1.0, 0.8, 0.55, 1.0]},
    {"id": "rays", "name": "Starburst Rays", "type": "slider", "default": 12, "min": 0, "max": 64, "sliderMax": 32, "decimals": 0},
    {"id": "ray_amount", "name": "Starburst Amount (%)", "type": "slider", "default": 60, "min": 0, "max": 200, "decimals": 1},
    {"id": "streak", "name": "Anamorphic Streak (%)", "type": "slider", "default": 50, "min": 0, "max": 200, "decimals": 1},
    {"id": "streak_color", "name": "Streak Color", "type": "color", "default": [0.5, 0.7, 1.0, 1.0]},
    {"id": "halo", "name": "Halo (%)", "type": "slider", "default": 30, "min": 0, "max": 200, "decimals": 1},
    {"id": "ghosts", "name": "Ghosts (%)", "type": "slider", "default": 50, "min": 0, "max": 200, "decimals": 1},
    {"id": "seed", "name": "Ghost Seed", "type": "slider", "default": 0, "min": 0, "max": 10000, "sliderMax": 100, "decimals": 0},
    {"id": "flare_only", "name": "Flare Only", "type": "checkbox", "default": false}
  ]
}"#;

evil_common::export_plugin!(MANIFEST, 19, render);

const GHOSTS: usize = 6;

fn render(p: &[f64; 19], px: &mut [Px], w: usize, h: usize, _time: f64, _scale: f64) {
    let f = p.map(|v| v as f32);
    let (lx, ly, bright, size) = (f[0], f[1], f[2] / 100.0, f[3] / 100.0);
    let color = [f[4], f[5], f[6]];
    let (nrays, ray_amt, streak) = (f[8].round(), f[9] / 100.0, f[10] / 100.0);
    let scol = [f[11], f[12], f[13]];
    let (halo, ghosts, seed, only) = (f[15] / 100.0, f[16] / 100.0, p[17], f[18] != 0.0);
    if bright <= 0.0 {
        if only {
            px.fill([0.0; 4]);
        }
        return;
    }
    // Sizes scale with the frame's short side, so the flare looks the same at any resolution.
    let r = w.min(h) as f32 * 0.05 * size;
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    // Ghosts sit on the line from the light through the frame centre: position, radius, tint.
    let ghost: Vec<(f32, f32, f32, f32, [f32; 3])> = (0..GHOSTS)
        .map(|i| {
            let hsh = |k: f64| hash(i as f64 * 13.0 + k, seed) as f32;
            let t = 0.4 + hsh(1.0) * 1.8; // 0 = light, 1 = centre, 2 = mirrored light
            let gr = r * (0.3 + hsh(2.0) * 2.5);
            let tint = [
                0.4 + hsh(3.0) * 0.6,
                0.4 + hsh(4.0) * 0.6,
                0.4 + hsh(5.0) * 0.6,
            ];
            (
                lx + (cx - lx) * t,
                ly + (cy - ly) * t,
                gr,
                0.15 + hsh(6.0) * 0.25,
                tint,
            )
        })
        .collect();
    let streak_len = w as f32 * 0.4 * size;
    let phase = hash(99.0, seed) as f32 * std::f32::consts::TAU;
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x as f32 + 0.5 - lx, y as f32 + 0.5 - ly);
            let d = dx.hypot(dy) / r;
            let mut add = [0.0f32; 3];
            // Core: bright centre with a soft falloff.
            let core = (-d * d).exp() + 0.15 / (1.0 + d * d * 0.1);
            // Starburst: angular spikes that fade with distance.
            let burst = if nrays > 0.0 && ray_amt > 0.0 {
                let a = dy.atan2(dx) * nrays / 2.0 + phase; // |cos| peaks twice per turn
                ray_amt * a.cos().abs().powf(8.0) * 0.6 / (1.0 + d * d * 0.03)
            } else {
                0.0
            };
            for k in 0..3 {
                add[k] += (core + burst) * color[k];
            }
            // Anamorphic streak: spans the frame, at least a pixel thick so it never aliases away.
            if streak > 0.0 {
                let s = streak
                    * (-(dy / (r * 0.15).max(1.0)).powi(2)).exp()
                    * (-(dx / streak_len).abs()).exp();
                (0..3).for_each(|k| add[k] += s * scol[k]);
            }
            // Halo: thin ring around the light.
            if halo > 0.0 {
                let ring = halo * 0.4 * (-((d - 6.0) / 0.6).powi(2)).exp();
                (0..3).for_each(|k| add[k] += ring * color[k]);
            }
            if ghosts > 0.0 {
                for &(gx, gy, gr, ga, tint) in &ghost {
                    let gd = (x as f32 + 0.5 - gx).hypot(y as f32 + 0.5 - gy) / gr;
                    // Soft-edged disc, a little brighter at the rim.
                    let disc = ((1.0 - gd) * 6.0).clamp(0.0, 1.0) * (0.6 + 0.4 * gd.min(1.0));
                    (0..3).for_each(|k| add[k] += ghosts * ga * disc * tint[k] * color[k]);
                }
            }
            let c = &mut px[y * w + x];
            if only {
                *c = [0.0; 4];
            }
            let a = add.map(|v| v * bright);
            (0..3).for_each(|k| c[k] += a[k]);
            c[3] += (1.0 - c[3]) * a.iter().fold(0.0f32, |m, v| m.max(*v)).clamp(0.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(bright: f64, only: bool) -> [f64; 19] {
        #[rustfmt::skip]
        let p = [40.0, 30.0, bright, 100.0, 1.0, 0.8, 0.55, 1.0, 12.0, 60.0, 50.0, 0.5, 0.7, 1.0, 1.0, 30.0, 50.0, 3.0, only as u8 as f64];
        p
    }

    fn run(p: [f64; 19]) -> Vec<Px> {
        let mut px = vec![[0.0, 0.0, 0.0, 1.0]; 160 * 90];
        render(&p, &mut px, 160, 90, 0.0, 1.0);
        px
    }

    #[test]
    fn zero_brightness_is_identity() {
        assert_eq!(
            run(params(0.0, false)),
            vec![[0.0, 0.0, 0.0, 1.0]; 160 * 90]
        );
    }

    #[test]
    fn brightest_at_the_light_and_streak_reaches_sideways() {
        let px = run(params(100.0, false));
        let at = |x: usize, y: usize| px[y * 160 + x][0];
        assert!(at(40, 30) > at(60, 50));
        assert!(at(40, 30) > 0.5);
        // The anamorphic streak lights the light's row far to the side, but not far above it.
        let mut no_streak = params(100.0, false);
        no_streak[10] = 0.0;
        let base = run(no_streak);
        let gain = |x: usize, y: usize| at(x, y) - base[y * 160 + x][0];
        assert!(gain(100, 30) > 0.05, "{}", gain(100, 30));
        assert!(gain(100, 10) < 1e-3, "{}", gain(100, 10));
    }

    #[test]
    fn flare_only_clears_the_layer_and_seed_is_deterministic() {
        let a = run(params(100.0, true));
        assert_eq!(a, run(params(100.0, true)));
        assert!(a.iter().all(|c| c.iter().all(|v| v.is_finite())));
    }
}
