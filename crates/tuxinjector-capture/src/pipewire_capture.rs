// PipeWire window capture for Wayland.
// Opens a portal session for the window picker, then runs a PipeWire stream
// on a background thread pulling video frames into shared memory.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::portal::{self, PortalSession};
use crate::CapturedFrame;

struct SharedFrame {
    data: Option<CapturedFrame>,
    updated: Instant,
}

struct CaptureSession {
    shared: Arc<Mutex<SharedFrame>>,
    _thread: std::thread::JoinHandle<()>,
    _portal: PortalSession,
}

// one portal session + pw stream thread per window.
// not pretty but keeps sessions isolated from each other
pub struct PipeWireCapture {
    sessions: std::collections::HashMap<String, CaptureSession>,
}

impl PipeWireCapture {
    pub fn new() -> Self {
        Self {
            sessions: std::collections::HashMap::new(),
        }
    }

    // starts capture for `id`. shows portal picker if no restore token.
    // returns the new persist token on success (for skipping the picker next time)
    pub fn start_capture(
        &mut self,
        id: &str,
        restore_token: Option<&str>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        if self.sessions.contains_key(id) {
            tracing::debug!(id, "capture already running");
            return Ok(None);
        }

        tracing::info!(id, "starting portal screencast session");
        let portal_sess = portal::start_screencast(restore_token)?;
        let node = portal_sess.node_id;
        let token = portal_sess.restore_token.clone();

        let shared = Arc::new(Mutex::new(SharedFrame {
            data: None,
            updated: Instant::now(),
        }));

        let shared2 = Arc::clone(&shared);
        let cap_id = id.to_string();

        let handle = std::thread::Builder::new()
            .name(format!("pw-capture-{id}"))
            .spawn(move || {
                if let Err(e) = run_pw_stream(node, shared2) {
                    tracing::error!(id = cap_id, error = %e, "PipeWire stream failed");
                }
            })
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        self.sessions.insert(
            id.to_string(),
            CaptureSession {
                shared,
                _thread: handle,
                _portal: portal_sess,
            },
        );

        Ok(token)
    }

    pub fn latest_frame(&self, id: &str) -> Option<CapturedFrame> {
        let sess = self.sessions.get(id)?;
        let lock = sess.shared.lock().ok()?;
        let frame = lock.data.as_ref()?;
        // TODO: use Arc<CapturedFrame> to avoid this clone
        Some(CapturedFrame {
            pixels: frame.pixels.clone(),
            width: frame.width,
            height: frame.height,
        })
    }

    pub fn stop_capture(&mut self, id: &str) {
        if self.sessions.remove(id).is_some() {
            tracing::info!(id, "stopped PipeWire capture");
        }
    }

    pub fn active_sessions(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    // "receiving" means we got a frame within the last 2 seconds
    pub fn is_receiving(&self, id: &str) -> bool {
        self.sessions
            .get(id)
            .and_then(|s| s.shared.lock().ok())
            .map(|g| g.updated.elapsed() < Duration::from_secs(2))
            .unwrap_or(false)
    }
}

// --- pw stream thread ---

fn run_pw_stream(
    node_id: u32,
    shared: Arc<Mutex<SharedFrame>>,
) -> Result<(), Box<dyn std::error::Error>> {
    pipewire::init();

    let mainloop = pipewire::main_loop::MainLoop::new(None)?;
    let ctx = pipewire::context::Context::new(&mainloop)?;
    let core = ctx.connect(None)?;

    let stream = pipewire::stream::Stream::new(
        &core,
        "tuxinjector-capture",
        pipewire::properties::properties! {
            *pipewire::keys::MEDIA_TYPE => "Video",
            *pipewire::keys::MEDIA_CATEGORY => "Capture",
            *pipewire::keys::MEDIA_ROLE => "Screen",
        },
    )?;

    let shared_cb = Arc::clone(&shared);

    let _listener = stream
        .add_local_listener_with_user_data(())
        .param_changed(|_, _ud, param_id, _param| {
            if param_id == pipewire::spa::param::ParamType::Format.as_raw() {
                tracing::debug!("PipeWire format negotiated");
            }
        })
        .process(move |stream, _ud| {
            if let Some(mut buf) = stream.dequeue_buffer() {
                ingest_buffer(&mut buf, &shared_cb);
            }
        })
        .register()?;

    // hook into the portal's node
    stream.connect(
        pipewire::spa::utils::Direction::Input,
        Some(node_id),
        pipewire::stream::StreamFlags::AUTOCONNECT | pipewire::stream::StreamFlags::MAP_BUFFERS,
        &mut [],
    )?;

    tracing::info!(node_id, "PipeWire stream connected");
    mainloop.run();

    Ok(())
}

// grab pixel data from a pw buffer and swizzle BGRA -> RGBA for GL
fn ingest_buffer(
    buffer: &mut pipewire::buffer::Buffer<'_>,
    shared: &Arc<Mutex<SharedFrame>>,
) {
    let datas = buffer.datas_mut();
    if datas.is_empty() {
        return;
    }

    let plane = &mut datas[0];

    // read chunk metadata before we borrow the pixel slice
    let chunk_sz = plane.chunk().size() as usize;
    let stride = plane.chunk().stride() as usize;

    if chunk_sz == 0 || stride == 0 {
        return;
    }

    let Some(raw) = plane.data() else { return };

    if chunk_sz > raw.len() {
        return;
    }
    let raw = &raw[..chunk_sz];

    let bpp = 4;
    let w = stride / bpp;
    let h = chunk_sz / stride;

    if w == 0 || h == 0 {
        return;
    }

    // BGRA -> RGBA swizzle
    let mut rgba = Vec::with_capacity(w * h * 4);
    for row in 0..h {
        let start = row * stride;
        let end = start + w * bpp;
        if end > raw.len() {
            break;
        }
        for px in raw[start..end].chunks_exact(4) {
            rgba.push(px[2]); // R
            rgba.push(px[1]); // G
            rgba.push(px[0]); // B
            rgba.push(px[3]); // A
        }
    }

    if let Ok(mut g) = shared.lock() {
        g.data = Some(CapturedFrame {
            pixels: rgba,
            width: w as u32,
            height: h as u32,
        });
        g.updated = Instant::now();
    }
}
