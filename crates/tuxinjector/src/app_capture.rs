// Grabs companion app pixels via X11 GetImage and draws them as GL textures
// inside the game's backbuffer. We try XComposite redirect first to keep the
// window invisible, and fall back to override_redirect + offscreen positioning
// if composite isn't available (or on XWayland where it's flaky).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use x11rb::connection::Connection;
use x11rb::protocol::composite::ConnectionExt as CompositeExt;
use x11rb::protocol::res::{ClientIdMask, ClientIdSpec, ConnectionExt as ResExt};
use x11rb::protocol::xproto::{
    AtomEnum, ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt as XprotoExt,
    ImageFormat, Pixmap, Window,
};
use x11rb::rust_connection::RustConnection;

type X11Res<T> = Result<T, Box<dyn std::error::Error>>;

const CAPTURE_INTERVAL: Duration = Duration::from_millis(100);

// Java Swing apps need a bit of time to stabilize after we find the window
const STABILIZATION_FRAMES: u64 = 120;

pub struct CapturedApp {
    pub pixels: Vec<u8>, // RGBA
    pub width: u32,
    pub height: u32,
    pub anchor_x: f32,  // viewport-relative
    pub anchor_y: f32,
}

enum CapturePhase {
    // window found, unmapped, waiting for things to settle
    Stabilizing { found_at_frame: u64, unmapped: bool },
    // XComposite redirect active, grabbing via named pixmap
    Composited { pixmap: Pixmap },
    // fallback: override_redirect + offscreen, using GetImage directly
    Offscreen,
}

struct EmbeddedWindow {
    window: Window,
    phase: CapturePhase,
    pixels: Option<Vec<u8>>,  // cached RGBA (already BGRA->RGBA converted)
    width: u32,
    height: u32,
    last_capture: Instant,
    sibling_windows: Vec<Window>,  // all other windows from this PID (kept unmapped)
}

pub struct AppCaptureManager {
    conn: Option<RustConnection>,
    screen_num: usize,
    wm_pid_atom: u32,
    has_composite: Option<bool>,
    embedded: HashMap<u32, EmbeddedWindow>,
    search_fails: HashMap<u32, u32>,
    frame: u64,
    visible: bool,
}

impl AppCaptureManager {
    pub fn new() -> Self {
        Self {
            conn: None,
            screen_num: 0,
            wm_pid_atom: 0,
            has_composite: None,
            embedded: HashMap::new(),
            search_fails: HashMap::new(),
            frame: 0,
            visible: false,
        }
    }

    pub fn known_pids(&self) -> Vec<u32> {
        self.embedded.keys().copied().collect()
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
        tracing::info!(visible = self.visible, "toggled anchored app visibility");
    }

    pub fn drop_window(&mut self, pid: u32) {
        if let Some(entry) = self.embedded.remove(&pid) {
            if let Some(conn) = self.conn.as_ref() {
                match &entry.phase {
                    CapturePhase::Composited { pixmap } => {
                        let _ = conn.free_pixmap(*pixmap);
                        let _ = conn.composite_unredirect_window(
                            entry.window,
                            x11rb::protocol::composite::Redirect::MANUAL,
                        );
                        let _ = conn.flush();
                    }
                    CapturePhase::Offscreen => {
                        let _ = conn.unmap_window(entry.window);
                        let _ = conn.flush();
                    }
                    CapturePhase::Stabilizing { .. } => {}
                }
            }
        }
        self.search_fails.remove(&pid);
    }

    // Main entry point - finds the window, grabs pixels, returns them for rendering.
    // Returns None if we haven't found it yet or if the overlay is hidden.
    pub fn embed(
        &mut self,
        pid: u32,
        vp_w: u32,
        vp_h: u32,
        anchor: tuxinjector_gui::running_apps::Anchor,
    ) -> Option<CapturedApp> {
        self.frame = self.frame.wrapping_add(1);

        // back off after 500 failed searches
        // NOTE: This probably can be safely decreased
        let fails = self.search_fails.get(&pid).copied().unwrap_or(0);
        if fails > 500 && self.frame % 60 != 0 {
            return None;
        }

        if !self.ensure_connected() {
            return None;
        }

        // discover the app window if we haven't cached it yet
        if !self.embedded.contains_key(&pid) {
            let all_wins = self.find_all_windows_by_pid(pid);
            if all_wins.is_empty() {
                let f = self.search_fails.entry(pid).or_insert(0);
                *f = f.saturating_add(1);
                if *f == 1 || *f % 10 == 0 {
                    tracing::debug!(pid, failures = *f, "companion app window not found yet");
                }
                return None;
            }

            self.search_fails.remove(&pid);

            // largest window = capture target
            let win = all_wins[0];
            let siblings: Vec<Window> = all_wins[1..].to_vec();

            // unmap everything so nothing stays visible on screen
            let mut unmapped = false;
            if let Some(conn) = self.conn.as_ref() {
                for &w in &all_wins {
                    if let Ok(cookie) = conn.unmap_window(w) {
                        if cookie.check().is_ok() {
                            tracing::debug!(pid, win = w, "unmapped companion window");
                        }
                    }
                }
                let _ = conn.flush();
                unmapped = true;
            }

            tracing::debug!(pid, capture_win = win, sibling_count = siblings.len(), "discovered all windows for PID");

            self.embedded.insert(
                pid,
                EmbeddedWindow {
                    window: win,
                    phase: CapturePhase::Stabilizing {
                        found_at_frame: self.frame,
                        unmapped,
                    },
                    pixels: None,
                    width: 0,
                    height: 0,
                    last_capture: Instant::now() - CAPTURE_INTERVAL,
                    sibling_windows: siblings,
                },
            );
        } else {
            // periodically check for late sibling windows (Java apps love doing this)
            if self.frame % 120 == 0 {
                self.unmap_new_siblings(pid);
            }
        }

        // state machine: handle stabilization -> active transition
        let frame = self.frame;
        let has_composite = self.has_composite.unwrap_or(false);
        {
            let conn = self.conn.as_ref()?;
            let entry = self.embedded.get(&pid)?;
            if let CapturePhase::Stabilizing { found_at_frame, unmapped } = &entry.phase {
                let age = frame.wrapping_sub(*found_at_frame);
                if age < STABILIZATION_FRAMES {
                    return None;
                }

                // make sure the window didn't vanish while we waited
                if conn
                    .get_window_attributes(entry.window)
                    .ok()
                    .and_then(|c| c.reply().ok())
                    .is_none()
                {
                    tracing::debug!(pid, win = entry.window, "window vanished during stabilization");
                    self.embedded.remove(&pid);
                    return None;
                }

                let unmapped = *unmapped;
                let win = entry.window;

                // try composite first, fall back to offscreen
                let new_phase = if has_composite {
                    match setup_composite(conn, win, unmapped) {
                        Ok(pixmap) => {
                            tracing::info!(pid, win, pixmap, "XComposite capture active");
                            Some(CapturePhase::Composited { pixmap })
                        }
                        Err(e) => {
                            tracing::warn!(pid, win, %e, "XComposite failed, trying offscreen");
                            match setup_offscreen(conn, win, unmapped) {
                                Ok(()) => {
                                    tracing::info!(pid, win, "offscreen capture active (fallback)");
                                    Some(CapturePhase::Offscreen)
                                }
                                Err(e2) => {
                                    tracing::warn!(pid, win, %e2, "offscreen fallback also failed");
                                    None
                                }
                            }
                        }
                    }
                } else {
                    match setup_offscreen(conn, win, unmapped) {
                        Ok(()) => {
                            tracing::info!(pid, win, "offscreen capture active (no composite)");
                            Some(CapturePhase::Offscreen)
                        }
                        Err(e) => {
                            tracing::warn!(pid, win, %e, "offscreen setup failed");
                            None
                        }
                    }
                };

                match new_phase {
                    Some(phase) => {
                        let _ = entry;
                        self.embedded.get_mut(&pid)?.phase = phase;
                    }
                    None => {
                        self.embedded.remove(&pid);
                        let f = self.search_fails.entry(pid).or_insert(0);
                        *f = f.saturating_add(1);
                        return None;
                    }
                }
            }
        }

        // skip pixel capture when hidden, but discovery/setup above still runs
        if !self.visible {
            return None;
        }

        // rate-limited pixel capture
        {
            let conn = self.conn.as_ref()?;
            let entry = self.embedded.get_mut(&pid)?;
            do_capture(pid, conn, entry);
        }

        // return whatever we've got cached
        let entry = self.embedded.get(&pid)?;
        let pixels = entry.pixels.as_ref()?;
        if entry.width == 0 || entry.height == 0 {
            return None;
        }

        let (off_x, off_y) = anchor.position(
            vp_w as i32, vp_h as i32,
            entry.width as i32, entry.height as i32,
            0,
        );

        Some(CapturedApp {
            pixels: pixels.clone(),
            width: entry.width,
            height: entry.height,
            anchor_x: off_x as f32,
            anchor_y: off_y as f32,
        })
    }

    fn ensure_connected(&mut self) -> bool {
        if self.conn.is_some() {
            return true;
        }
        let x_disp = std::env::var("DISPLAY").unwrap_or_else(|_| "<unset>".into());
        match x11rb::connect(None) {
            Ok((conn, screen)) => {
                tracing::debug!(%x_disp, screen, "AppCaptureManager connected to X11");
                let composite = probe_composite(&conn);
                self.conn = Some(conn);
                self.screen_num = screen;
                self.has_composite = Some(composite);
                true
            }
            Err(e) => {
                tracing::warn!(%x_disp, %e, "failed to connect to X11");
                false
            }
        }
    }

    fn net_wm_pid_atom(&mut self) -> Option<u32> {
        if self.wm_pid_atom != 0 {
            return Some(self.wm_pid_atom);
        }
        let conn = self.conn.as_ref()?;
        match conn.intern_atom(false, b"_NET_WM_PID") {
            Ok(c) => match c.reply() {
                Ok(r) => {
                    self.wm_pid_atom = r.atom;
                    Some(r.atom)
                }
                Err(_) => None,
            },
            Err(_) => None,
        }
    }

    // Find ALL windows belonging to a PID, sorted largest-first
    fn find_all_windows_by_pid(&mut self, pid: u32) -> Vec<Window> {
        // try XRes first (fast)
        if let Some(conn) = self.conn.as_ref() {
            let wins = find_all_via_xres(conn, self.screen_num, pid);
            if !wins.is_empty() {
                return wins;
            }
        }

        // fallback: _NET_WM_PID tree walk
        let atom = match self.net_wm_pid_atom() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let conn = match self.conn.as_ref() {
            Some(c) => c,
            None => return Vec::new(),
        };
        let root = conn.setup().roots[self.screen_num].root;
        let mut results = Vec::new();
        find_all_recursive(conn, root, atom, pid, &mut results);
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.into_iter().map(|(w, _)| w).collect()
    }

    // Check for new sibling windows from the same PID and unmap them
    fn unmap_new_siblings(&mut self, pid: u32) {
        let entry = match self.embedded.get_mut(&pid) {
            Some(e) => e,
            None => return,
        };
        let capture_win = entry.window;
        let known: Vec<Window> = std::iter::once(capture_win)
            .chain(entry.sibling_windows.iter().copied())
            .collect();

        let all_wins = if let Some(conn) = self.conn.as_ref() {
            find_all_via_xres(conn, self.screen_num, pid)
        } else {
            return;
        };

        let entry = match self.embedded.get_mut(&pid) {
            Some(e) => e,
            None => return,
        };

        for w in &all_wins {
            if !known.contains(w) {
                if let Some(conn) = self.conn.as_ref() {
                    let _ = conn.unmap_window(*w);
                    let _ = conn.flush();
                    tracing::debug!(pid, win = *w, "unmapped late sibling window");
                }
                entry.sibling_windows.push(*w);
            }
        }
    }
}

// free fn to sidestep borrow conflicts with &mut self
fn do_capture(pid: u32, conn: &RustConnection, entry: &mut EmbeddedWindow) {
    let now = Instant::now();
    if now.duration_since(entry.last_capture) < CAPTURE_INTERVAL && entry.pixels.is_some() {
        return;
    }

    let (w, h) = match win_size(conn, entry.window) {
        Some(wh) => wh,
        None => {
            tracing::debug!(pid, win = entry.window, "window geometry query failed");
            return;
        }
    };
    if w == 0 || h == 0 {
        return;
    }

    // handle resize: need to recreate the composite pixmap
    if (w != entry.width || h != entry.height) && entry.width != 0 {
        if let CapturePhase::Composited { pixmap } = &entry.phase {
            let _ = conn.free_pixmap(*pixmap);
            match conn.generate_id() {
                Ok(new_pm) => {
                    if conn
                        .composite_name_window_pixmap(entry.window, new_pm)
                        .ok()
                        .and_then(|c| c.check().ok())
                        .is_some()
                    {
                        entry.phase = CapturePhase::Composited { pixmap: new_pm };
                        tracing::debug!(pid, w, h, "recreated composite pixmap after resize");
                    }
                }
                Err(_) => {}
            }
        }
    }

    let drawable = match &entry.phase {
        CapturePhase::Composited { pixmap } => *pixmap as u32,
        CapturePhase::Offscreen => entry.window,
        CapturePhase::Stabilizing { .. } => return,
    };

    match conn.get_image(ImageFormat::Z_PIXMAP, drawable, 0, 0, w as u16, h as u16, !0) {
        Ok(cookie) => match cookie.reply() {
            Ok(reply) => {
                entry.pixels = Some(bgra_to_rgba(&reply.data, reply.depth));
                entry.width = w;
                entry.height = h;
                entry.last_capture = now;
            }
            Err(e) => {
                tracing::debug!(pid, win = entry.window, %e, "GetImage reply failed");
            }
        },
        Err(e) => {
            tracing::debug!(pid, win = entry.window, %e, "GetImage request failed");
        }
    }
}

// Check if XComposite >= 0.2 is available (needed for NameWindowPixmap)
fn probe_composite(conn: &RustConnection) -> bool {
    match conn.composite_query_version(0, 4) {
        Ok(cookie) => match cookie.reply() {
            Ok(reply) => {
                let ok = reply.major_version > 0 || reply.minor_version >= 2;
                tracing::info!(
                    major = reply.major_version,
                    minor = reply.minor_version,
                    ok,
                    "XComposite probe"
                );
                ok
            }
            Err(_) => {
                tracing::debug!("XComposite query_version reply failed");
                false
            }
        },
        Err(_) => {
            tracing::debug!("XComposite not available");
            false
        }
    }
}

// XComposite redirect: window renders to offscreen pixmap, invisible on screen.
// NOTE: We also set override_redirect + offscreen position because on XWayland,
// Redirect::MANUAL alone doesn't prevent the Wayland compositor from showing it.
fn setup_composite(conn: &RustConnection, win: Window, already_unmapped: bool) -> X11Res<Pixmap> {
    conn.change_window_attributes(
        win,
        &ChangeWindowAttributesAux::new().override_redirect(1),
    )?.check()?;
    conn.configure_window(
        win,
        &ConfigureWindowAux::new().x(-32000).y(-32000),
    )?.check()?;

    // redirect before mapping so it goes straight to the offscreen pixmap
    conn.composite_redirect_window(
        win,
        x11rb::protocol::composite::Redirect::MANUAL,
    )?.check()?;

    // map so the app actually renders
    if already_unmapped {
        conn.map_window(win)?.check()?;
    }

    let pixmap = conn.generate_id()?;
    conn.composite_name_window_pixmap(win, pixmap)?.check()?;
    conn.flush()?;
    Ok(pixmap)
}

fn setup_offscreen(conn: &RustConnection, win: Window, already_unmapped: bool) -> X11Res<()> {
    if !already_unmapped {
        conn.unmap_window(win)?.check()?;
    }

    conn.change_window_attributes(
        win,
        &ChangeWindowAttributesAux::new().override_redirect(1),
    )?.check()?;

    conn.configure_window(
        win,
        &ConfigureWindowAux::new().x(-32000).y(-32000),
    )?.check()?;

    conn.map_window(win)?.check()?;
    conn.flush()?;
    Ok(())
}

// X11 gives us BGRA (Z_PIXMAP, little-endian), we need RGBA.
// 24-bit depth windows don't have real alpha, so we force 255.
fn bgra_to_rgba(data: &[u8], depth: u8) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(data.len());
    for chunk in data.chunks_exact(4) {
        rgba.push(chunk[2]); // R
        rgba.push(chunk[1]); // G
        rgba.push(chunk[0]); // B
        rgba.push(if depth >= 32 { chunk[3] } else { 255 });
    }
    rgba
}

fn win_size(conn: &RustConnection, window: Window) -> Option<(u32, u32)> {
    let g = conn.get_geometry(window).ok()?.reply().ok()?;
    Some((g.width as u32, g.height as u32))
}

// XRes-based window search - faster than tree-walking _NET_WM_PID
fn find_all_via_xres(
    conn: &RustConnection,
    screen_num: usize,
    target_pid: u32,
) -> Vec<Window> {
    let specs = [ClientIdSpec {
        client: 0,
        mask: ClientIdMask::LOCAL_CLIENT_PID,
    }];
    let reply = match conn.res_query_client_ids(&specs).ok().and_then(|c| c.reply().ok()) {
        Some(r) => r,
        None => return Vec::new(),
    };

    let id_mask = conn.setup().resource_id_mask;

    let matching_bases: Vec<u32> = reply
        .ids
        .iter()
        .filter(|id| id.value.first().copied() == Some(target_pid))
        .map(|id| id.spec.client)
        .collect();

    if matching_bases.is_empty() {
        return Vec::new();
    }

    tracing::debug!(target_pid, clients = matching_bases.len(), "XRes found matching clients");

    let root = conn.setup().roots[screen_num].root;
    let mut candidates: Vec<(Window, u32)> = Vec::new();

    if let Some(tree) = conn.query_tree(root).ok().and_then(|c| c.reply().ok()).map(|r| r.children) {
        // check top-level children first
        for &child in &tree {
            if matching_bases.contains(&(child & !id_mask)) {
                if let Some(area) = input_output_area(conn, child) {
                    candidates.push((child, area));
                }
            }
        }
        // if nothing at top level, recurse
        if candidates.is_empty() {
            for &child in &tree {
                collect_matching(conn, child, &matching_bases, id_mask, &mut candidates);
            }
        }
    }

    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    if !candidates.is_empty() {
        tracing::debug!(target_pid, total = candidates.len(), "XRes: found windows");
    }
    candidates.into_iter().map(|(w, _)| w).collect()
}

fn input_output_area(conn: &RustConnection, window: Window) -> Option<u32> {
    use x11rb::protocol::xproto::WindowClass;
    let attrs = conn.get_window_attributes(window).ok()?.reply().ok()?;
    // HACK: GLFW creates InputOnly windows that we need to skip
    if attrs.class == WindowClass::INPUT_OUTPUT {
        let geom = conn.get_geometry(window).ok()?.reply().ok()?;
        Some(geom.width as u32 * geom.height as u32)
    } else {
        None
    }
}

fn collect_matching(
    conn: &RustConnection,
    window: Window,
    bases: &[u32],
    id_mask: u32,
    out: &mut Vec<(Window, u32)>,
) {
    if bases.contains(&(window & !id_mask)) {
        if let Some(area) = input_output_area(conn, window) {
            out.push((window, area));
        }
    }
    if let Some(tree) = conn.query_tree(window).ok().and_then(|c| c.reply().ok()) {
        for child in tree.children {
            collect_matching(conn, child, bases, id_mask, out);
        }
    }
}

// fallback: walk the whole X tree checking _NET_WM_PID on each window
fn find_all_recursive(
    conn: &RustConnection,
    window: Window,
    wm_pid_atom: u32,
    target_pid: u32,
    out: &mut Vec<(Window, u32)>,
) {
    if let Ok(cookie) = conn.get_property(false, window, wm_pid_atom, AtomEnum::CARDINAL, 0, 1) {
        if let Ok(reply) = cookie.reply() {
            if let Some(pid) = reply.value32().and_then(|mut it| it.next()) {
                if pid == target_pid {
                    let area = input_output_area(conn, window).unwrap_or(0);
                    out.push((window, area));
                }
            }
        }
    }

    if let Some(tree) = conn.query_tree(window).ok().and_then(|c| c.reply().ok()) {
        for child in tree.children {
            find_all_recursive(conn, child, wm_pid_atom, target_pid, out);
        }
    }
}
