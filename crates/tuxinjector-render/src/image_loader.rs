// Image loading (PNG, JPEG, GIF) into RGBA pixel buffers.
// GIFs get decoded into animation frames with per-frame delays.

use std::path::Path;
use std::time::Duration;

pub struct LoadedImage {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub struct AnimationFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub delay: Duration,
}

pub enum ImageData {
    Static(LoadedImage),
    Animated {
        frames: Vec<AnimationFrame>,
        loop_count: u32, // 0 = loop forever
    },
}

impl ImageData {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            ImageData::Static(img) => (img.width, img.height),
            ImageData::Animated { frames, .. } => {
                frames.first().map(|f| (f.width, f.height)).unwrap_or((0, 0))
            }
        }
    }

    pub fn is_animated(&self) -> bool {
        matches!(self, ImageData::Animated { frames, .. } if frames.len() > 1)
    }
}

// Load an image from disk. Format auto-detected from extension.
pub fn load_image(path: &Path) -> Result<ImageData, Box<dyn std::error::Error>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if ext == "gif" {
        return load_gif(path);
    }

    load_static(path)
}

fn load_static(path: &Path) -> Result<ImageData, Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();

    Ok(ImageData::Static(LoadedImage {
        pixels: rgba.into_raw(),
        width: w,
        height: h,
    }))
}

fn load_gif(path: &Path) -> Result<ImageData, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::BufReader;

    let file = BufReader::new(File::open(path)?);
    let decoder = image::codecs::gif::GifDecoder::new(file)?;

    use image::AnimationDecoder;
    let raw_frames: Vec<image::Frame> = decoder.into_frames().collect_frames()?;

    if raw_frames.len() <= 1 {
        // single-frame gif, just treat it as static
        if let Some(frame) = raw_frames.into_iter().next() {
            let buf = frame.into_buffer();
            let (w, h) = buf.dimensions();
            return Ok(ImageData::Static(LoadedImage {
                pixels: buf.into_raw(),
                width: w,
                height: h,
            }));
        }
        return Err("GIF has no frames".into());
    }

    let frames: Vec<AnimationFrame> = raw_frames
        .into_iter()
        .map(|frame: image::Frame| {
            let delay = frame.delay();
            let (num, denom) = delay.numer_denom_ms();
            let ms = if denom == 0 { 100 } else { num / denom };
            let ms = ms.max(20); // browsers cap at 20ms minimum, so do we

            let buf = frame.into_buffer();
            let (w, h) = buf.dimensions();
            AnimationFrame {
                pixels: buf.into_raw(),
                width: w,
                height: h,
                delay: Duration::from_millis(ms as u64),
            }
        })
        .collect();

    Ok(ImageData::Animated {
        frames,
        loop_count: 0,
    })
}

// Tracks playback position for animated images
pub struct AnimationPlayer {
    n_frames: usize,
    cur: usize,
    accum: Duration,
    delays: Vec<Duration>,
    max_loops: u32,
    loops_done: u32,
}

impl AnimationPlayer {
    pub fn new(frames: &[AnimationFrame], max_loops: u32) -> Self {
        Self {
            n_frames: frames.len(),
            cur: 0,
            accum: Duration::ZERO,
            delays: frames.iter().map(|f| f.delay).collect(),
            max_loops,
            loops_done: 0,
        }
    }

    // Tick forward by dt, returns the frame index to display
    pub fn advance(&mut self, dt: Duration) -> Option<usize> {
        if self.n_frames == 0 {
            return None;
        }

        // already finished all loops
        if self.max_loops > 0 && self.loops_done >= self.max_loops {
            return Some(self.n_frames - 1);
        }

        self.accum += dt;

        while self.accum >= self.delays[self.cur] {
            self.accum -= self.delays[self.cur];
            self.cur += 1;

            if self.cur >= self.n_frames {
                self.cur = 0;
                self.loops_done += 1;

                if self.max_loops > 0 && self.loops_done >= self.max_loops {
                    self.cur = self.n_frames - 1;
                    return Some(self.cur);
                }
            }
        }

        Some(self.cur)
    }

    pub fn reset(&mut self) {
        self.cur = 0;
        self.accum = Duration::ZERO;
        self.loops_done = 0;
    }

    pub fn current_frame(&self) -> usize {
        self.cur
    }
}

// Zero out the alpha channel for pixels matching key_color within sensitivity.
// Works in sRGB space since that's what we get from the game.
pub fn apply_color_key(
    pixels: &mut [u8],
    key: [f32; 3],
    sensitivity: f32,
) {
    let thresh_sq = sensitivity * sensitivity;

    for px in pixels.chunks_exact_mut(4) {
        let r = px[0] as f32 / 255.0;
        let g = px[1] as f32 / 255.0;
        let b = px[2] as f32 / 255.0;

        let dr = r - key[0];
        let dg = g - key[1];
        let db = b - key[2];

        if dr * dr + dg * dg + db * db < thresh_sq {
            px[3] = 0;
        }
    }
}

// Apply multiple color keys in one pass. First match wins.
pub fn apply_color_keys(
    pixels: &mut [u8],
    keys: &[([f32; 3], f32)],
) {
    for px in pixels.chunks_exact_mut(4) {
        let r = px[0] as f32 / 255.0;
        let g = px[1] as f32 / 255.0;
        let b = px[2] as f32 / 255.0;

        for &(key, sens) in keys {
            let dr = r - key[0];
            let dg = g - key[1];
            let db = b - key[2];

            if dr * dr + dg * dg + db * db < sens * sens {
                px[3] = 0;
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_key_makes_matching_pixels_transparent() {
        let mut pixels = vec![
            0, 0, 0, 255,     // black - should match
            255, 0, 0, 255,   // red - shouldn't match
        ];

        apply_color_key(&mut pixels, [0.0, 0.0, 0.0], 0.1);

        assert_eq!(pixels[3], 0);     // black gone
        assert_eq!(pixels[7], 255);   // red untouched
    }

    #[test]
    fn animation_player_loops() {
        let frames = vec![
            AnimationFrame { pixels: vec![], width: 1, height: 1, delay: Duration::from_millis(100) },
            AnimationFrame { pixels: vec![], width: 1, height: 1, delay: Duration::from_millis(100) },
        ];

        let mut player = AnimationPlayer::new(&frames, 0);

        assert_eq!(player.advance(Duration::from_millis(50)), Some(0));
        assert_eq!(player.advance(Duration::from_millis(60)), Some(1));  // 110ms -> past frame 0
        assert_eq!(player.advance(Duration::from_millis(50)), Some(1));  // 60ms into frame 1
        assert_eq!(player.advance(Duration::from_millis(50)), Some(0));  // wraps around
    }

    #[test]
    fn animation_player_finite_loops() {
        let frames = vec![
            AnimationFrame { pixels: vec![], width: 1, height: 1, delay: Duration::from_millis(100) },
        ];

        let mut player = AnimationPlayer::new(&frames, 2);

        assert_eq!(player.advance(Duration::from_millis(110)), Some(0));  // loop 1
        assert_eq!(player.advance(Duration::from_millis(110)), Some(0));  // loop 2
        assert_eq!(player.advance(Duration::from_millis(110)), Some(0));  // done, stays on last
    }
}
