use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EasingType {
    Linear,
    EaseOut,
    EaseIn,
    EaseInOut,
}

impl Default for EasingType {
    fn default() -> Self {
        Self::Linear
    }
}

// Apply easing curve to a normalized t in [0, 1].
// in_pow / out_pow control how aggressive the curve is.
pub fn ease(t: f32, easing: EasingType, in_pow: f32, out_pow: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match easing {
        EasingType::Linear => t,
        EasingType::EaseIn => t.powf(in_pow),
        EasingType::EaseOut => 1.0 - (1.0 - t).powf(out_pow),
        EasingType::EaseInOut => {
            if t < 0.5 {
                // first half: ease-in mapped to [0, 0.5]
                0.5 * (2.0 * t).powf(in_pow)
            } else {
                // second half: ease-out mapped to [0.5, 1.0]
                1.0 - 0.5 * (2.0 * (1.0 - t)).powf(out_pow)
            }
        }
    }
}

// Damped sinusoidal bounce from 0 -> 1.
// The oscillation rides on top of a linear ramp and fades out as t approaches 1.
// TODO: maybe let the user configure decay rate separately from intensity
pub fn bounce(t: f32, num_bounces: i32, intensity: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if num_bounces <= 0 || intensity <= 0.0 {
        return t;
    }

    let pi = std::f32::consts::PI;
    let osc = (num_bounces as f32 * pi * t).sin();
    let decay = (-(intensity * t)).exp();
    t + osc * decay * (1.0 - t)
}

// Tracks a width/height animation, used for mode switching and eyezoom resize.
#[derive(Debug, Clone)]
pub struct TransitionState {
    pub active: bool,
    pub started_at: Instant,
    pub duration_ms: u32,
    pub progress: f32,
    pub from_w: i32,
    pub from_h: i32,
    pub to_w: i32,
    pub to_h: i32,
}

impl Default for TransitionState {
    fn default() -> Self {
        Self {
            active: false,
            started_at: Instant::now(),
            duration_ms: 0,
            progress: 0.0,
            from_w: 0, from_h: 0,
            to_w: 0, to_h: 0,
        }
    }
}

impl TransitionState {
    pub fn start(&mut self, from_w: i32, from_h: i32, to_w: i32, to_h: i32, duration_ms: u32) {
        self.active = true;
        self.started_at = Instant::now();
        self.duration_ms = duration_ms;
        self.progress = 0.0;
        self.from_w = from_w;
        self.from_h = from_h;
        self.to_w = to_w;
        self.to_h = to_h;
    }

    // Advance the animation. Returns true while it's still running.
    pub fn update(&mut self) -> bool {
        if !self.active {
            return false;
        }

        let elapsed_ms = self.started_at.elapsed().as_millis() as f32;
        let dur = self.duration_ms as f32;

        if dur <= 0.0 {
            // zero duration = instant snap
            self.progress = 1.0;
            self.active = false;
        } else {
            self.progress = (elapsed_ms / dur).min(1.0);
            if self.progress >= 1.0 {
                self.active = false;
            }
        }
        self.active
    }

    // Interpolated (w, h) at current progress with easing applied
    pub fn current_size(&self, easing: EasingType, in_pow: f32, out_pow: f32) -> (i32, i32) {
        let t = ease(self.progress, easing, in_pow, out_pow);
        let dw = (self.to_w - self.from_w) as f32;
        let dh = (self.to_h - self.from_h) as f32;
        let w = self.from_w as f32 + dw * t;
        let h = self.from_h as f32 + dh * t;
        (w.round() as i32, h.round() as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_ease() {
        assert!((ease(0.0, EasingType::Linear, 2.0, 2.0) - 0.0).abs() < 1e-6);
        assert!((ease(0.5, EasingType::Linear, 2.0, 2.0) - 0.5).abs() < 1e-6);
        assert!((ease(1.0, EasingType::Linear, 2.0, 2.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ease_in_quadratic() {
        let v = ease(0.5, EasingType::EaseIn, 2.0, 2.0);
        assert!((v - 0.25).abs() < 1e-6); // 0.5^2 = 0.25
    }

    #[test]
    fn ease_out_quadratic() {
        let v = ease(0.5, EasingType::EaseOut, 2.0, 2.0);
        assert!((v - 0.75).abs() < 1e-6); // 1 - (0.5)^2 = 0.75
    }

    #[test]
    fn ease_in_out_endpoints() {
        assert!((ease(0.0, EasingType::EaseInOut, 2.0, 2.0) - 0.0).abs() < 1e-6);
        assert!((ease(1.0, EasingType::EaseInOut, 2.0, 2.0) - 1.0).abs() < 1e-6);
        // midpoint should be ~0.5
        assert!((ease(0.5, EasingType::EaseInOut, 2.0, 2.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn bounce_endpoints() {
        assert!((bounce(0.0, 3, 5.0) - 0.0).abs() < 1e-4);
        // at t=1, sin(n*pi) = 0 so we land right on 1.0
        assert!((bounce(1.0, 3, 5.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn clamp_out_of_range() {
        assert!((ease(-1.0, EasingType::Linear, 2.0, 2.0) - 0.0).abs() < 1e-6);
        assert!((ease(2.0, EasingType::Linear, 2.0, 2.0) - 1.0).abs() < 1e-6);
    }
}
