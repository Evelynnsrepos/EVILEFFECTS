//! Evil Flicker: random brightness changes over time.

use evil_common::{Px, hash, noise};

const MANIFEST: &str = r#"{
  "api": 1,
  "id": "com.evileffects.flicker",
  "name": "Evil Flicker",
  "category": "EVILEFFECTS",
  "version": "0.1.0",
  "author": "EVILEFFECTS contributors",
  "description": "Random brightness flicker over time.",
  "params": [
    {"id": "amount", "name": "Amount", "type": "slider", "default": 50, "min": 0, "max": 100, "decimals": 1},
    {"id": "speed", "name": "Flickers per Second", "type": "slider", "default": 12, "min": 0, "max": 120, "sliderMax": 60, "decimals": 1},
    {"id": "min_brightness", "name": "Minimum Brightness", "type": "slider", "default": 0, "min": 0, "max": 100, "decimals": 1},
    {"id": "smooth", "name": "Smooth", "type": "checkbox", "default": true},
    {"id": "seed", "name": "Random Seed", "type": "slider", "default": 0, "min": 0, "max": 10000, "sliderMax": 100, "decimals": 0}
  ]
}"#;

evil_common::export_plugin!(MANIFEST, 5, render);

fn render(p: &[f64; 5], px: &mut [Px], _w: usize, _h: usize, time: f64, _scale: f64) {
    let [amount, speed, min_b, smooth, seed] = *p;
    let t = time * speed;
    // Smooth eases between random values; off jumps to a new one each flicker.
    let n = if smooth != 0.0 {
        noise(t, seed)
    } else {
        hash(t.floor(), seed) * 2.0 - 1.0
    };
    let gain = (1.0 + amount / 100.0 * n).max(min_b / 100.0) as f32;
    for c in px.iter_mut() {
        for v in &mut c[..3] {
            *v *= gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(p: [f64; 5], time: f64) -> Px {
        let mut px = vec![[0.5, 0.5, 0.5, 1.0]];
        render(&p, &mut px, 1, 1, time, 1.0);
        px[0]
    }

    #[test]
    fn zero_amount_is_identity() {
        assert_eq!(run([0.0, 12.0, 0.0, 1.0, 0.0], 0.37), [0.5, 0.5, 0.5, 1.0]);
    }

    #[test]
    fn flickers_over_time_and_keeps_alpha() {
        let vals: Vec<f32> = (0..50)
            .map(|i| run([100.0, 12.0, 0.0, 0.0, 7.0], i as f64 / 24.0)[0])
            .collect();
        assert!(
            vals.iter().any(|v| *v < 0.4) && vals.iter().any(|v| *v > 0.6),
            "{vals:?}"
        );
        assert_eq!(run([100.0, 12.0, 0.0, 0.0, 7.0], 0.5)[3], 1.0);
    }

    #[test]
    fn minimum_brightness_holds() {
        for i in 0..50 {
            assert!(run([100.0, 12.0, 80.0, 0.0, 1.0], i as f64 / 24.0)[0] >= 0.4 - 1e-6);
        }
    }
}
