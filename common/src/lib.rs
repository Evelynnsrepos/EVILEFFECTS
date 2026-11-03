//! Shared code for the EVILEFFECTS plug-ins: the EffectCraft plug-in ABI (v1) exports and a few
//! pixel helpers. Pixels are premultiplied RGBA `f32`, row major.

use std::cell::RefCell;

pub type Px = [f32; 4];

/// Most image samples per pixel an effect takes.
/// shortcut: EffectCraft gives a plug-in 20 000 fuel per pixel and one sample with mirrored edges costs ~700,
/// so 24 leaves headroom; raise it if the host's fuel budget grows or sampling gets cheaper.
pub const MAX_SAMPLES: usize = 24;

thread_local! {
    static BUF: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

/// One reusable, 8-byte-aligned buffer for the parameters and pixels (`ec_alloc`).
pub fn alloc(bytes: i32) -> i32 {
    BUF.with(|b| {
        let mut b = b.borrow_mut();
        let words = (bytes.max(0) as usize).div_ceil(8);
        if b.len() < words {
            b.resize(words, 0);
        }
        b.as_mut_ptr() as i32
    })
}

/// Exports the plug-in ABI for `$manifest` (a JSON `&str`) and
/// `$render(params: &[f64; $nparams], pixels: &mut [Px], width, height, time, scale)`.
/// `$nparams` is the flattened parameter count (point = 2, color = 4); a test checks it against the manifest.
#[macro_export]
macro_rules! export_plugin {
    ($manifest:expr, $nparams:expr, $render:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn ec_api_version() -> i32 {
            1
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ec_manifest_ptr() -> i32 {
            $manifest.as_ptr() as i32
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ec_manifest_len() -> i32 {
            $manifest.len() as i32
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ec_alloc(bytes: i32) -> i32 {
            $crate::alloc(bytes)
        }

        /// # Safety
        /// The host passes pointers into the buffer `ec_alloc` returned.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ec_render(
            pixels: i32,
            width: i32,
            height: i32,
            params: i32,
            nparams: i32,
            time: f64,
            scale: f64,
        ) -> i32 {
            if nparams < $nparams || width <= 0 || height <= 0 {
                return 1;
            }
            let (w, h) = (width as usize, height as usize);
            let p = unsafe { &*(params as *const [f64; $nparams]) };
            let px = unsafe { std::slice::from_raw_parts_mut(pixels as *mut $crate::Px, w * h) };
            $render(p, px, w, h, time, scale);
            0
        }

        #[cfg(test)]
        #[test]
        fn manifest_matches_param_count() {
            let m: serde_json::Value = serde_json::from_str($manifest).expect("manifest is JSON");
            assert_eq!(m["api"], 1);
            let n: usize = m["params"]
                .as_array()
                .expect("params")
                .iter()
                .map(|p| match p["type"].as_str() {
                    Some("point") => 2,
                    Some("color") => 4,
                    _ => 1,
                })
                .sum();
            assert_eq!(n, $nparams);
        }
    };
}

pub fn luma(c: &Px) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// Bilinear sample at pixel coordinates (pixel centres at +0.5), clamped to the edge.
pub fn sample(px: &[Px], w: usize, h: usize, x: f32, y: f32) -> Px {
    let x = (x - 0.5).clamp(0.0, (w - 1) as f32);
    let y = (y - 0.5).clamp(0.0, (h - 1) as f32);
    let (x0, y0) = (x as usize, y as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |xx: usize, yy: usize| px[yy * w + xx];
    let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
    std::array::from_fn(|i| {
        let top = a[i] + (b[i] - a[i]) * fx;
        let bot = c[i] + (d[i] - c[i]) * fx;
        top + (bot - top) * fy
    })
}

/// [`sample`] with an edge mode, the manifests' `Edges` popup: 0 transparent, 1 repeat edge, 2 mirror.
pub fn sample_edges(px: &[Px], w: usize, h: usize, x: f32, y: f32, mode: f64) -> Px {
    let (fw, fh) = (w as f32, h as f32);
    match mode as i32 {
        0 if x < 0.0 || y < 0.0 || x > fw || y > fh => [0.0; 4],
        2 => {
            let m = |v: f32, n: f32| {
                let v = v.rem_euclid(2.0 * n);
                if v > n { 2.0 * n - v } else { v }
            };
            sample(px, w, h, m(x, fw), m(y, fh))
        }
        _ => sample(px, w, h, x, y),
    }
}

/// Approximate gaussian blur: three box blurs per axis, `radius` in pixels.
pub fn blur(px: &mut [Px], w: usize, h: usize, radius: f32) {
    let r = (radius / 3.0).round() as usize;
    if r == 0 {
        return;
    }
    let mut line = Vec::new();
    for _ in 0..3 {
        for y in 0..h {
            box_line(px, y * w, 1, w, r, &mut line);
        }
        for x in 0..w {
            box_line(px, x, w, h, r, &mut line);
        }
    }
}

/// One box blur along `n` pixels starting at `start` with `stride`, edges clamped.
fn box_line(px: &mut [Px], start: usize, stride: usize, n: usize, r: usize, line: &mut Vec<Px>) {
    line.clear();
    line.extend((0..n).map(|i| px[start + i * stride]));
    let get = |i: isize| line[i.clamp(0, n as isize - 1) as usize];
    let inv = 1.0 / (2 * r + 1) as f32;
    let mut sum = [0.0f32; 4];
    for i in -(r as isize)..=(r as isize) {
        let c = get(i);
        (0..4).for_each(|k| sum[k] += c[k]);
    }
    for i in 0..n {
        px[start + i * stride] = sum.map(|s| s * inv);
        let (add, sub) = (
            get(i as isize + r as isize + 1),
            get(i as isize - r as isize),
        );
        (0..4).for_each(|k| sum[k] += add[k] - sub[k]);
    }
}

/// Deterministic hash of an integer step and seed to [0, 1).
pub fn hash(n: f64, seed: f64) -> f64 {
    ((n * 127.1 + seed * 311.7).sin() * 43758.5453).rem_euclid(1.0)
}

/// Smooth 1D value noise in [-1, 1].
pub fn noise(t: f64, seed: f64) -> f64 {
    let (i, f) = (t.floor(), t - t.floor());
    let s = f * f * (3.0 - 2.0 * f);
    (hash(i, seed) + (hash(i + 1.0, seed) - hash(i, seed)) * s) * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_keeps_energy_and_spreads() {
        let (w, h) = (31, 31);
        let mut px = vec![[0.0; 4]; w * h];
        px[15 * w + 15] = [1.0; 4];
        blur(&mut px, w, h, 9.0);
        let total: f32 = px.iter().map(|p| p[0]).sum();
        assert!((total - 1.0).abs() < 1e-3, "{total}");
        assert!(px[15 * w + 18][0] > 0.0);
    }

    #[test]
    fn noise_is_bounded_and_deterministic() {
        for i in 0..1000 {
            let t = i as f64 * 0.137;
            let n = noise(t, 3.0);
            assert!((-1.0..=1.0).contains(&n));
            assert_eq!(n, noise(t, 3.0));
        }
    }
}
