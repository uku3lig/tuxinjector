// Capture backends - PipeWire on Wayland, X11 fallback.
// Everything funnels into CapturedFrame (RGBA pixels for GL upload).

#[cfg(feature = "pipewire")]
pub mod pipewire_capture;
#[cfg(feature = "pipewire")]
mod portal;

pub struct CapturedFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

// just shells out to pw-cli to see if pipewire is alive
#[cfg(feature = "pipewire")]
pub fn pipewire_available() -> bool {
    std::process::Command::new("pw-cli")
        .arg("info")
        .arg("0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(feature = "pipewire"))]
pub fn pipewire_available() -> bool {
    false
}
