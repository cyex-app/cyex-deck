//! U6 The mention chime, made from its numbers: 880 Hz, 1.320 Hz after 90 ms; the level
//! rises from 0,0001 to 0,12 in 10 ms and falls back to 0,0001 at 300 ms (exponential
//! ramps, as Web Audio drew it before V1051-01); 320 ms long.

use super::RATE;

pub fn chime() -> Vec<f32> {
    let frames = (RATE as f32 * 0.32) as usize;
    let mut out = Vec::with_capacity(frames * 2);
    let mut phase = 0f32;
    for i in 0..frames {
        let t = i as f32 / RATE as f32;
        let freq = if t < 0.09 { 880.0 } else { 1320.0 };
        phase += std::f32::consts::TAU * freq / RATE as f32;
        let level = if t < 0.01 {
            0.0001 * (0.12f32 / 0.0001).powf(t / 0.01)
        } else if t < 0.3 {
            0.12 * (0.0001f32 / 0.12).powf((t - 0.01) / 0.29)
        } else {
            0.0
        };
        let s = phase.sin() * level;
        out.extend([s, s]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_320_ms_of_quiet_stereo() {
        let c = chime();
        assert_eq!(c.len(), 15_360 * 2);
        assert!(c.iter().all(|s| s.abs() <= 0.1201));
        assert!(c[0].abs() < 0.001 && c[c.len() - 1] == 0.0);
    }
}
