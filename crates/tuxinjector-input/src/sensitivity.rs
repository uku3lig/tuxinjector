// Mouse sensitivity scaling with layered overrides.
// Priority: hotkey > mode > base config value.

use tracing::trace;

pub struct SensitivityState {
    base: f32,
    // (uniform, optional per-axis (x, y))
    mode_override: Option<(f32, Option<(f32, f32)>)>,
    hotkey_override: Option<(f32, Option<(f32, f32)>)>,
    // last output position (what the game actually sees)
    out_x: f64,
    out_y: f64,
    // last raw input position from GLFW
    raw_x: f64,
    raw_y: f64,
    inited: bool,
}

impl SensitivityState {
    pub fn new() -> Self {
        Self {
            base: 1.0,
            mode_override: None,
            hotkey_override: None,
            out_x: 0.0,
            out_y: 0.0,
            raw_x: 0.0,
            raw_y: 0.0,
            inited: false,
        }
    }

    pub fn set_base_sensitivity(&mut self, s: f32) {
        tracing::debug!(sensitivity = s, "sensitivity: set_base_sensitivity");
        self.base = s;
    }

    pub fn set_mode_override(&mut self, s: f32, separate: Option<(f32, f32)>) {
        self.mode_override = Some((s, separate));
    }

    pub fn clear_mode_override(&mut self) {
        self.mode_override = None;
    }

    // toggles the hotkey override on/off
    pub fn toggle_hotkey_override(&mut self, s: f32, separate: Option<(f32, f32)>) {
        if self.hotkey_override.is_some() {
            self.hotkey_override = None;
            trace!("hotkey sensitivity override cleared");
        } else {
            self.hotkey_override = Some((s, separate));
            trace!(
                sensitivity = s,
                ?separate,
                "hotkey sensitivity override enabled"
            );
        }
    }

    pub fn has_hotkey_override(&self) -> bool {
        self.hotkey_override.is_some()
    }

    // scale cursor position using delta from last raw input
    pub fn scale_cursor(&mut self, x: f64, y: f64) -> (f64, f64) {
        if !self.inited {
            self.raw_x = x;
            self.raw_y = y;
            self.out_x = x;
            self.out_y = y;
            self.inited = true;
            return (x, y);
        }

        let dx = x - self.raw_x;
        let dy = y - self.raw_y;

        let (sx, sy) = self.effective_sensitivity();

        tracing::debug!(
            raw_x = x, raw_y = y,
            dx, dy,
            sens_x = sx, sens_y = sy,
            "scale_cursor: raw GLFW delta"
        );

        let nx = self.out_x + dx * sx as f64;
        let ny = self.out_y + dy * sy as f64;

        self.raw_x = x;
        self.raw_y = y;
        self.out_x = nx;
        self.out_y = ny;

        (nx, ny)
    }

    pub fn reset_tracking(&mut self) {
        self.inited = false;
    }

    fn effective_sensitivity(&self) -> (f32, f32) {
        // hotkey takes priority over everything
        if let Some((uniform, separate)) = self.hotkey_override {
            return match separate {
                Some((x, y)) => (x, y),
                None => (uniform, uniform),
            };
        }

        if let Some((uniform, separate)) = self.mode_override {
            return match separate {
                Some((x, y)) => (x, y),
                None => (uniform, uniform),
            };
        }

        (self.base, self.base)
    }

    pub fn get_effective_sensitivity(&self) -> (f32, f32) {
        self.effective_sensitivity()
    }
}

impl Default for SensitivityState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sensitivity_is_identity() {
        let mut s = SensitivityState::new();

        let (x, y) = s.scale_cursor(100.0, 200.0);
        assert_eq!((x, y), (100.0, 200.0));

        let (x, y) = s.scale_cursor(110.0, 220.0);
        assert!((x - 110.0).abs() < 1e-10);
        assert!((y - 220.0).abs() < 1e-10);
    }

    #[test]
    fn base_sensitivity_scales_delta() {
        let mut s = SensitivityState::new();
        s.set_base_sensitivity(2.0);

        s.scale_cursor(100.0, 100.0);

        let (x, y) = s.scale_cursor(110.0, 120.0);
        assert!((x - 120.0).abs() < 1e-10, "frame1 x: {x}"); // 100 + 10*2
        assert!((y - 140.0).abs() < 1e-10, "frame1 y: {y}"); // 100 + 20*2

        let (x2, y2) = s.scale_cursor(120.0, 140.0);
        assert!((x2 - 140.0).abs() < 1e-10, "frame2 x: {x2}"); // 120 + 10*2
        assert!((y2 - 180.0).abs() < 1e-10, "frame2 y: {y2}"); // 140 + 20*2

        let (x3, y3) = s.scale_cursor(130.0, 160.0);
        assert!((x3 - 160.0).abs() < 1e-10, "frame3 x: {x3}");
        assert!((y3 - 220.0).abs() < 1e-10, "frame3 y: {y3}");
    }

    #[test]
    fn mode_override_takes_precedence() {
        let mut s = SensitivityState::new();
        s.set_base_sensitivity(2.0);
        s.set_mode_override(0.5, None);

        s.scale_cursor(100.0, 100.0);

        let (x, y) = s.scale_cursor(110.0, 120.0);
        assert!((x - 105.0).abs() < 1e-10); // 100 + 10*0.5
        assert!((y - 110.0).abs() < 1e-10); // 100 + 20*0.5
    }

    #[test]
    fn hotkey_override_takes_top_priority() {
        let mut s = SensitivityState::new();
        s.set_base_sensitivity(2.0);
        s.set_mode_override(0.5, None);
        s.toggle_hotkey_override(3.0, None);

        s.scale_cursor(100.0, 100.0);

        let (x, y) = s.scale_cursor(110.0, 120.0);
        assert!((x - 130.0).abs() < 1e-10); // 100 + 10*3
        assert!((y - 160.0).abs() < 1e-10); // 100 + 20*3
    }

    #[test]
    fn toggle_hotkey_override_on_off() {
        let mut s = SensitivityState::new();
        s.set_base_sensitivity(1.0);

        assert!(!s.has_hotkey_override());

        s.toggle_hotkey_override(0.5, None);
        assert!(s.has_hotkey_override());

        s.toggle_hotkey_override(0.5, None);
        assert!(!s.has_hotkey_override());
    }

    #[test]
    fn separate_xy_sensitivity() {
        let mut s = SensitivityState::new();
        s.set_mode_override(1.0, Some((0.5, 2.0)));

        s.scale_cursor(100.0, 100.0);

        let (x, y) = s.scale_cursor(120.0, 110.0);
        assert!((x - 110.0).abs() < 1e-10); // 100 + 20*0.5
        assert!((y - 120.0).abs() < 1e-10); // 100 + 10*2.0
    }

    #[test]
    fn reset_tracking_reinitializes() {
        let mut s = SensitivityState::new();
        s.set_base_sensitivity(2.0);

        s.scale_cursor(100.0, 100.0);
        s.scale_cursor(110.0, 110.0);

        s.reset_tracking();

        let (x, y) = s.scale_cursor(200.0, 200.0);
        assert_eq!((x, y), (200.0, 200.0));
    }
}
