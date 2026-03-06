// Lua runtime thread -- owns the VM, runs callbacks, handles hot-reload.
// On reload the entire VM is destroyed and recreated from scratch (stateless).

use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

use crossbeam_channel::{Receiver, Sender, bounded, select};
use mlua::prelude::*;
use tuxinjector_config::Config;

use crate::actions::{LuaActionBinding, TuxinjectorCommand};
use crate::api::{self, ApiState};
use crate::loader;

// Config + bindings that come out of evaluating a Lua config
pub struct LuaConfigUpdate {
    pub config: Config,
    pub bindings: Vec<LuaActionBinding>,
}

// Handle to a running Lua runtime thread
pub struct LuaRuntime {
    pub callback_tx: Sender<u64>,
    pub state_event_tx: Sender<String>,
    command_rx: Receiver<TuxinjectorCommand>,
    reload_tx: Sender<String>,
    config_rx: Receiver<Result<LuaConfigUpdate, String>>,
}

impl LuaRuntime {
    // Spawn the runtime thread with initial config source. Blocks until
    // the first eval is done so we have a config to work with right away.
    pub fn spawn(source: String) -> Result<(Self, LuaConfigUpdate), String> {
        let (cb_tx, cb_rx) = bounded(64);
        let (state_tx, state_rx) = bounded(64);
        let (cmd_tx, cmd_rx) = bounded(64);
        let (reload_tx, reload_rx) = bounded(4);
        let (cfg_tx, cfg_rx) = bounded::<Result<LuaConfigUpdate, String>>(4);

        thread::Builder::new()
            .name("lua-runtime".into())
            .spawn(move || {
                runtime_loop(source, cb_rx, state_rx, cmd_tx, reload_rx, cfg_tx);
            })
            .map_err(|e| format!("failed to spawn Lua runtime thread: {e}"))?;

        // Wait for the initial config evaluation
        let initial = cfg_rx
            .recv()
            .map_err(|_| "Lua runtime thread died during init".to_string())?
            .map_err(|e| format!("Lua config evaluation failed: {e}"))?;

        Ok((
            Self {
                callback_tx: cb_tx,
                state_event_tx: state_tx,
                command_rx: cmd_rx,
                reload_tx,
                config_rx: cfg_rx,
            },
            initial,
        ))
    }

    // Reload config from new source. Blocks until eval is done.
    // On failure the old VM stays alive.
    pub fn reload(&self, source: String) -> Result<LuaConfigUpdate, String> {
        self.reload_tx
            .send(source)
            .map_err(|_| "Lua runtime thread is dead".to_string())?;
        self.config_rx
            .recv()
            .map_err(|_| "Lua runtime thread died during reload".to_string())?
    }

    // Drain any pending commands from Lua callbacks. Call this per-frame.
    pub fn drain_commands(&self) -> Vec<TuxinjectorCommand> {
        let mut cmds = Vec::new();
        while let Ok(cmd) = self.command_rx.try_recv() {
            cmds.push(cmd);
        }
        cmds
    }
}

fn runtime_loop(
    initial_src: String,
    cb_rx: Receiver<u64>,
    state_rx: Receiver<String>,
    cmd_tx: Sender<TuxinjectorCommand>,
    reload_rx: Receiver<String>,
    cfg_tx: Sender<Result<LuaConfigUpdate, String>>,
) {
    // Bootstrap: create VM and eval the initial config
    let (mut lua, mut api_st) = match load_and_setup(&initial_src, &cmd_tx) {
        Ok((l, a, config, bindings)) => {
            let _ = cfg_tx.send(Ok(LuaConfigUpdate { config, bindings }));
            (l, a)
        }
        Err(e) => {
            let _ = cfg_tx.send(Err(e));
            return;
        }
    };

    tracing::info!("Lua runtime thread started");

    loop {
        select! {
            recv(cb_rx) -> msg => {
                match msg {
                    Ok(cb_id) => {
                        run_callback(&lua, &api_st, cb_id, &cmd_tx);
                    }
                    Err(_) => {
                        tracing::info!("callback channel closed, shutting down lua runtime");
                        break;
                    }
                }
            }
            recv(state_rx) -> msg => {
                if let Ok(new_state) = msg {
                    fire_state_listeners(&lua, &api_st, &new_state, &cmd_tx);
                }
            }
            recv(reload_rx) -> msg => {
                match msg {
                    Ok(new_src) => {
                        tracing::info!("Lua runtime: reloading config");

                        // flush stale callbacks that were queued for the old VM
                        while cb_rx.try_recv().is_ok() {}

                        match load_and_setup(&new_src, &cmd_tx) {
                            Ok((new_lua, new_api, config, bindings)) => {
                                if cfg_tx.send(Ok(LuaConfigUpdate { config, bindings })).is_ok() {
                                    lua = new_lua;
                                    api_st = new_api;
                                    tracing::info!("Lua runtime: reload successful");
                                } else {
                                    tracing::error!("Lua runtime: config channel closed");
                                }
                            }
                            Err(e) => {
                                tracing::error!("Lua reload failed: {e} -- keeping old VM");
                                let _ = cfg_tx.send(Err(e));
                            }
                        }
                    }
                    Err(_) => {
                        tracing::info!("reload channel closed, shutting down lua runtime");
                        break;
                    }
                }
            }
        }
    }
}

// Create a fresh VM, install our API, evaluate the config source
fn load_and_setup(
    src: &str,
    cmd_tx: &Sender<TuxinjectorCommand>,
) -> Result<(Lua, Rc<RefCell<ApiState>>, Config, Vec<LuaActionBinding>), String> {
    let lua = loader::create_sandbox().map_err(|e| format!("sandbox creation failed: {e}"))?;
    let api_st = api::install_api(&lua).map_err(|e| format!("API install failed: {e}"))?;

    let val: LuaValue = lua
        .load(src)
        .eval()
        .map_err(|e| format!("Lua eval failed: {e}"))?;

    let config = match val {
        LuaValue::Table(_) => lua
            .from_value::<Config>(val)
            .map_err(|e| format!("config deserialization failed: {e}"))?,
        _ => return Err("init.lua must return a table".into()),
    };

    let st = api_st.borrow();
    let bindings = st.builder.bindings().to_vec();
    let init_cmds: Vec<_> = st.commands.clone();
    drop(st);

    // forward any commands that ran during config eval (e.g. top-level tx.switch_mode())
    for cmd in init_cmds {
        let _ = cmd_tx.try_send(cmd);
    }

    Ok((lua, api_st, config, bindings))
}

// Notify all registered state listeners about a state change
fn fire_state_listeners(
    lua: &Lua,
    api_st: &Rc<RefCell<ApiState>>,
    new_state: &str,
    cmd_tx: &Sender<TuxinjectorCommand>,
) {
    let n = api_st.borrow().state_listeners.len();
    for i in 0..n {
        // grab the function from registry, then drop the borrow before calling
        // so the callback can borrow_mut to push commands
        let func: LuaFunction = {
            let st = api_st.borrow();
            match lua.registry_value(&st.state_listeners[i]) {
                Ok(f) => f,
                Err(e) => {
                    tracing::error!(i, %e, "failed to get state listener from registry");
                    continue;
                }
            }
        };

        api_st.borrow_mut().commands.clear();

        if let Err(e) = func.call::<()>(new_state) {
            tracing::error!(%e, "state listener failed");
            continue;
        }

        // drain commands the listener produced and forward them
        let cmds: Vec<_> = api_st.borrow_mut().commands.drain(..).collect();
        for cmd in cmds {
            if let Err(e) = cmd_tx.try_send(cmd) {
                tracing::warn!(%e, "command channel full or closed in state listener");
            }
        }
    }
}

// Run a single callback by its ID
fn run_callback(
    lua: &Lua,
    api_st: &Rc<RefCell<ApiState>>,
    cb_id: u64,
    cmd_tx: &Sender<TuxinjectorCommand>,
) {
    let idx = cb_id as usize;

    // NOTE: we drop the borrow before calling the function, otherwise
    // the callback can't borrow_mut to push commands
    let func: LuaFunction = {
        let st = api_st.borrow();
        if idx >= st.callback_keys.len() {
            tracing::warn!(cb_id, "callback ID out of range");
            return;
        }
        match lua.registry_value(&st.callback_keys[idx]) {
            Ok(f) => f,
            Err(e) => {
                tracing::error!(cb_id, %e, "failed to get callback from registry");
                return;
            }
        }
    };

    api_st.borrow_mut().commands.clear();

    if let Err(e) = func.call::<()>(()) {
        tracing::error!(cb_id, %e, "Lua callback failed");
        return;
    }

    let cmds: Vec<_> = api_st.borrow_mut().commands.drain(..).collect();
    for cmd in cmds {
        if let Err(e) = cmd_tx.try_send(cmd) {
            tracing::warn!(%e, "command channel full or closed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_and_get_config() {
        let source = r#"
            return {
                display = { defaultMode = "Thin" },
                input = { mouseSensitivity = 0.5 },
            }
        "#;

        let (rt, update) = LuaRuntime::spawn(source.to_string()).unwrap();
        assert_eq!(update.config.display.default_mode, "Thin");
        assert!((update.config.input.mouse_sensitivity - 0.5).abs() < 1e-6);
        assert!(update.bindings.is_empty());
        drop(rt);
    }

    #[test]
    fn spawn_with_bindings() {
        let source = r#"
            local tx = require("tuxinjector")

            tx.bind("F1", function()
                tx.switch_mode("Thin")
            end)

            tx.bind("F2", function()
                tx.toggle_gui()
            end)

            return { display = { defaultMode = "Fullscreen" } }
        "#;

        let (rt, update) = LuaRuntime::spawn(source.to_string()).unwrap();
        assert_eq!(update.config.display.default_mode, "Fullscreen");
        assert_eq!(update.bindings.len(), 2);
        assert!(update.bindings[0].key_combo.contains(&290)); // F1
        assert!(update.bindings[1].key_combo.contains(&291)); // F2
        drop(rt);
    }

    #[test]
    fn callback_execution_produces_commands() {
        let source = r#"
            local tx = require("tuxinjector")

            tx.bind("F1", function()
                tx.switch_mode("Thin")
                tx.toggle_gui()
            end)

            return {}
        "#;

        let (rt, update) = LuaRuntime::spawn(source.to_string()).unwrap();
        assert_eq!(update.bindings.len(), 1);
        let cb_id = update.bindings[0].callback_id;

        rt.callback_tx.send(cb_id).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let cmds = rt.drain_commands();
        assert_eq!(cmds.len(), 2);
        assert!(matches!(&cmds[0], TuxinjectorCommand::SwitchMode(name) if name == "Thin"));
        assert!(matches!(&cmds[1], TuxinjectorCommand::ToggleGui));
    }

    #[test]
    fn reload_updates_config_and_bindings() {
        let src1 = r#"
            local tx = require("tuxinjector")
            tx.bind("F1", function() tx.switch_mode("A") end)
            return { display = { defaultMode = "First" } }
        "#;

        let (rt, update1) = LuaRuntime::spawn(src1.to_string()).unwrap();
        assert_eq!(update1.config.display.default_mode, "First");
        assert_eq!(update1.bindings.len(), 1);

        let src2 = r#"
            local tx = require("tuxinjector")
            tx.bind("F2", function() tx.switch_mode("B") end)
            tx.bind("F3", function() tx.toggle_gui() end)
            return { display = { defaultMode = "Second" } }
        "#;

        let update2 = rt.reload(src2.to_string()).unwrap();
        assert_eq!(update2.config.display.default_mode, "Second");
        assert_eq!(update2.bindings.len(), 2);
    }

    #[test]
    fn reload_failure_keeps_old_vm() {
        let src1 = r#"return { display = { defaultMode = "Good" } }"#;
        let (rt, update1) = LuaRuntime::spawn(src1.to_string()).unwrap();
        assert_eq!(update1.config.display.default_mode, "Good");

        let result = rt.reload("this is not valid lua!!!".to_string());
        assert!(result.is_err());

        // runtime should survive the bad reload and recover fine
        let update3 = rt.reload(r#"return { display = { defaultMode = "Recovered" } }"#.to_string()).unwrap();
        assert_eq!(update3.config.display.default_mode, "Recovered");
    }

    #[test]
    fn initial_commands_dispatched() {
        let source = r#"
            local tx = require("tuxinjector")
            tx.switch_mode("Thin")
            return {}
        "#;

        let (rt, _update) = LuaRuntime::spawn(source.to_string()).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        let cmds = rt.drain_commands();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], TuxinjectorCommand::SwitchMode(name) if name == "Thin"));
    }

    #[test]
    fn state_event_fires_listener() {
        let source = r#"
            local tx = require("tuxinjector")

            tx.listen("state", function(s)
                tx.switch_mode(s)
            end)

            return {}
        "#;

        let (rt, update) = LuaRuntime::spawn(source.to_string()).unwrap();
        assert_eq!(update.bindings.len(), 0);

        rt.state_event_tx.send("inworld".to_string()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let cmds = rt.drain_commands();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], TuxinjectorCommand::SwitchMode(name) if name == "inworld"));
    }

    #[test]
    fn multiple_state_listeners_all_fire() {
        let source = r#"
            local tx = require("tuxinjector")

            tx.listen("state", function(s) tx.switch_mode("A") end)
            tx.listen("state", function(s) tx.switch_mode("B") end)

            return {}
        "#;

        let (rt, _update) = LuaRuntime::spawn(source.to_string()).unwrap();

        rt.state_event_tx.send("wall".to_string()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let cmds = rt.drain_commands();
        assert_eq!(cmds.len(), 2);
    }
}
