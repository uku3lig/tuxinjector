// Config hot-reload via `notify`.
//
// Watches the config file (or the whole config dir for Lua require() support)
// and re-parses on changes, publishing the new snapshot through RCU.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{error, info, warn};

use crate::snapshot::ConfigSnapshot;
use crate::types::Config;

// 100ms debounce - vim/neovim do atomic rename-writes which
// fire multiple events back to back
const DEBOUNCE: Duration = Duration::from_millis(100);

// Takes config source text, returns parsed Config (or error string).
pub type ConfigParser = Box<dyn Fn(&str) -> Result<Config, String> + Send>;

pub struct ConfigWatcher {
    path: PathBuf,
    snap: Arc<ConfigSnapshot>,
    parser: ConfigParser,
    // when true, watch any .lua file in the dir (for require() deps)
    watch_all: bool,
    // NOTE: kept alive so the watcher doesn't get dropped
    _watcher: Option<RecommendedWatcher>,
}

impl ConfigWatcher {
    pub fn new(
        path: PathBuf,
        snap: Arc<ConfigSnapshot>,
        parser: ConfigParser,
    ) -> Result<Self, notify::Error> {
        Ok(Self {
            path,
            snap,
            parser,
            watch_all: false,
            _watcher: None,
        })
    }

    pub fn set_watch_all_files(&mut self, val: bool) {
        self.watch_all = val;
    }

    // Spawn the background watcher thread.
    pub fn start(&mut self) -> Result<(), notify::Error> {
        let path = self.path.clone();
        let snap = Arc::clone(&self.snap);
        let watch_all = self.watch_all;

        // Swap the parser out so we can move it into the thread
        let parser = std::mem::replace(&mut self.parser, Box::new(|_| Ok(Config::default())));

        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let mut watcher = notify::recommended_watcher(tx)?;

        // Watch the parent dir so we catch atomic-rename writes
        let watch_dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;

        info!("Config watcher started for {}", path.display());

        let fname = path
            .file_name()
            .map(|n| n.to_os_string())
            .unwrap_or_default();

        std::thread::Builder::new()
            .name("config-watcher".into())
            .spawn(move || {
                let mut last_reload = Instant::now() - DEBOUNCE * 2; // allow immediate first reload

                for evt in rx.iter() {
                    let evt = match evt {
                        Ok(e) => e,
                        Err(e) => {
                            warn!("File watch error: {e}");
                            continue;
                        }
                    };

                    // only care about writes/creates/renames
                    let dominated = matches!(
                        evt.kind,
                        EventKind::Modify(_) | EventKind::Create(_)
                    );
                    if !dominated { continue; }

                    // filter to our file, or any .lua when watching the whole dir
                    if !watch_all {
                        let ours = evt.paths.iter().any(|p| {
                            p.file_name()
                                .map(|n| n == fname)
                                .unwrap_or(false)
                        });
                        if !ours { continue; }
                    } else {
                        let is_lua = evt.paths.iter().any(|p| {
                            p.extension().map(|ext| ext == "lua").unwrap_or(false)
                        });
                        if !is_lua { continue; }
                    }

                    // debounce
                    let now = Instant::now();
                    if now.duration_since(last_reload) < DEBOUNCE {
                        continue;
                    }
                    last_reload = now;

                    // always re-eval the entry point, even when a helper module changed
                    match std::fs::read_to_string(&path) {
                        Ok(src) => match parser(&src) {
                            Ok(cfg) => {
                                info!("Config reloaded from {}", path.display());
                                snap.publish(cfg);
                            }
                            Err(e) => error!("Failed to parse config: {e}"),
                        },
                        Err(e) => error!("Failed to read config file: {e}"),
                    }
                }

                info!("Config watcher thread exiting");
            })
            .map_err(|e| notify::Error::generic(&e.to_string()))?;

        self._watcher = Some(watcher);
        Ok(())
    }
}
