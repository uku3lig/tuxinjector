// The `tuxinjector` Lua module. Scripts get it via require("tuxinjector").
//
// Config-time functions (like bind) accumulate state in ApiState.
// Runtime ones (like switch_mode) push commands into a vec that gets
// drained each frame on the render thread.
//
// TODO: Add more comments here for easier third party api support

use std::cell::RefCell;
use std::rc::Rc;

use mlua::prelude::*;

use crate::actions::{ActionBuilder, TuxinjectorCommand};
use crate::key_parse::parse_key_combo;

// Everything we accumulate while evaluating a Lua config
pub struct ApiState {
    pub builder: ActionBuilder,
    pub commands: Vec<TuxinjectorCommand>,
    // Registry keys keep Lua callbacks alive across GC. Indexed by callback_id.
    pub callback_keys: Vec<LuaRegistryKey>,
    pub state_listeners: Vec<LuaRegistryKey>,
}

impl ApiState {
    pub fn new() -> Self {
        Self {
            builder: ActionBuilder::new(),
            commands: Vec::new(),
            callback_keys: Vec::new(),
            state_listeners: Vec::new(),
        }
    }
}

// Wire up the `tuxinjector` module so require("tuxinjector") returns our API table.
pub fn install_api(lua: &Lua) -> LuaResult<Rc<RefCell<ApiState>>> {
    let st = Rc::new(RefCell::new(ApiState::new()));
    let tbl = lua.create_table()?;

    // tx.bind(keys, callback [, options])
    {
        let st = Rc::clone(&st);
        tbl.set(
            "bind",
            lua.create_function(move |lua, (keys, func, opts): (String, LuaFunction, Option<LuaTable>)| {
                let combo = parse_key_combo(&keys)
                    .map_err(|e| mlua::Error::external(e))?;

                let block = match opts {
                    Some(ref t) => t.get::<bool>("block").unwrap_or(true),
                    None => true,
                };

                // stash callback in registry so it survives GC
                let reg_key = lua.create_registry_value(func)?;

                let mut s = st.borrow_mut();
                let cb_id = s.builder.register(combo, block);

                // pad the vec so we can index directly by callback_id
                while s.callback_keys.len() <= cb_id as usize {
                    s.callback_keys.push(lua.create_registry_value(LuaNil)?);
                }
                s.callback_keys[cb_id as usize] = reg_key;

                Ok(())
            })?,
        )?;
    }

    // tx.switch_mode(name)
    {
        let st = Rc::clone(&st);
        tbl.set(
            "switch_mode",
            lua.create_function(move |_, name: String| {
                st.borrow_mut().commands.push(TuxinjectorCommand::SwitchMode(name));
                Ok(())
            })?,
        )?;
    }

    // tx.toggle_mode(main, fallback)
    {
        let st = Rc::clone(&st);
        tbl.set(
            "toggle_mode",
            lua.create_function(move |_, (main, fallback): (String, String)| {
                st.borrow_mut()
                    .commands
                    .push(TuxinjectorCommand::ToggleMode { main, fallback });
                Ok(())
            })?,
        )?;
    }

    // tx.set_sensitivity(s) -- 0.0 means "reset to config default"
    {
        let st = Rc::clone(&st);
        tbl.set(
            "set_sensitivity",
            lua.create_function(move |_, s: f32| {
                st.borrow_mut().commands.push(TuxinjectorCommand::SetSensitivity(s));
                Ok(())
            })?,
        )?;
    }

    // tx.toggle_gui()
    {
        let st = Rc::clone(&st);
        tbl.set(
            "toggle_gui",
            lua.create_function(move |_, ()| {
                st.borrow_mut().commands.push(TuxinjectorCommand::ToggleGui);
                Ok(())
            })?,
        )?;
    }

    // tx.exec(cmd) -- fire & forget, spawns a subprocess
    {
        let st = Rc::clone(&st);
        tbl.set(
            "exec",
            lua.create_function(move |_, cmd: String| {
                st.borrow_mut().commands.push(TuxinjectorCommand::Exec(cmd));
                Ok(())
            })?,
        )?;
    }

    // tx.toggle_app_visibility()
    {
        let st = Rc::clone(&st);
        tbl.set(
            "toggle_app_visibility",
            lua.create_function(move |_, ()| {
                st.borrow_mut().commands.push(TuxinjectorCommand::ToggleAppVisibility);
                Ok(())
            })?,
        )?;
    }

    // tx.press_key(keyname) -- synthetic press+release for each key in combo
    {
        let st = Rc::clone(&st);
        tbl.set(
            "press_key",
            lua.create_function(move |_, key: String| {
                let keys = parse_key_combo(&key)
                    .map_err(|e| mlua::Error::external(e))?;
                let mut s = st.borrow_mut();
                for &k in &keys {
                    s.commands.push(TuxinjectorCommand::PressKey(k));
                }
                Ok(())
            })?,
        )?;
    }

    // tx.get_key(keyname) -- true if every key in the combo is currently held
    tbl.set(
        "get_key",
        lua.create_function(|_, key: String| {
            let keys = parse_key_combo(&key)
                .map_err(|e| mlua::Error::external(e))?;
            let all_held = keys.iter().all(|&k| tuxinjector_input::is_key_pressed(k));
            Ok(all_held)
        })?,
    )?;

    // tx.sleep(ms) -- intentionally blocks the lua thread
    tbl.set(
        "sleep",
        lua.create_function(|_, ms: u64| {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            Ok(())
        })?,
    )?;

    // tx.log(msg)
    tbl.set(
        "log",
        lua.create_function(|_, msg: String| {
            tracing::info!(target: "lua", "{msg}");
            Ok(())
        })?,
    )?;

    // tx.state() -- returns current game state string
    tbl.set(
        "state",
        lua.create_function(|_, ()| {
            Ok(crate::get_game_state())
        })?,
    )?;

    // tx.get_mode()
    tbl.set(
        "get_mode",
        lua.create_function(|_, ()| {
            Ok(crate::get_mode_name())
        })?,
    )?;

    // tx.active_res() -> (w, h)
    tbl.set(
        "active_res",
        lua.create_function(|_, ()| {
            Ok(crate::get_active_res())
        })?,
    )?;

    // tx.current_time() -- millis since unix epoch
    tbl.set(
        "current_time",
        lua.create_function(|_, ()| {
            let ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            Ok(ms)
        })?,
    )?;

    // tx.listen(event, fn) -- subscribe to events. only "state" for now
    {
        let st = Rc::clone(&st);
        tbl.set(
            "listen",
            lua.create_function(move |lua, (event, func): (String, LuaFunction)| {
                match event.as_str() {
                    "state" => {
                        let key = lua.create_registry_value(func)?;
                        st.borrow_mut().state_listeners.push(key);
                        Ok(())
                    }
                    // TODO: maybe add a "mode" event at some point
                    _ => Err(mlua::Error::external(format!("unknown event: '{event}'"))),
                }
            })?,
        )?;
    }

    // shove it into package.loaded so require("tuxinjector") picks it up
    let loaded: LuaTable = lua.globals().get::<LuaTable>("package")?.get("loaded")?;
    loaded.set("tuxinjector", tbl)?;

    Ok(st)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_registers_action() {
        let lua = Lua::new();
        let state = install_api(&lua).unwrap();

        lua.load(
            r#"
            local tx = require("tuxinjector")
            tx.bind("ctrl+F1", function()
                tx.switch_mode("Thin")
            end)
            tx.bind("F2", function()
                tx.toggle_gui()
            end, { block = false })
            "#,
        )
        .exec()
        .unwrap();

        let st = state.borrow();
        assert_eq!(st.builder.bindings().len(), 2);
        assert_eq!(st.callback_keys.len(), 2);
    }

    #[test]
    fn runtime_commands_collected() {
        let lua = Lua::new();
        let state = install_api(&lua).unwrap();

        lua.load(
            r#"
            local tx = require("tuxinjector")
            tx.switch_mode("Tall")
            tx.set_sensitivity(0.5)
            tx.toggle_gui()
            tx.exec("echo hello")
            tx.log("test message")
            "#,
        )
        .exec()
        .unwrap();

        let st = state.borrow();
        assert_eq!(st.commands.len(), 4); // log goes to tracing, not commands
    }

    #[test]
    fn bind_with_invalid_key_errors() {
        let lua = Lua::new();
        let _state = install_api(&lua).unwrap();

        let result = lua.load(
            r#"
            local tx = require("tuxinjector")
            tx.bind("ctrl+banana", function() end)
            "#,
        ).exec();

        assert!(result.is_err());
    }

    #[test]
    fn state_returns_string() {
        let lua = Lua::new();
        let _state = install_api(&lua).unwrap();

        let result: String = lua.load(
            r#"
            local tx = require("tuxinjector")
            return tx.state()
            "#,
        ).eval().unwrap();

        let _ = result;
    }

    #[test]
    fn listen_registers_state_listener() {
        let lua = Lua::new();
        let state = install_api(&lua).unwrap();

        lua.load(
            r#"
            local tx = require("tuxinjector")
            tx.listen("state", function(s) end)
            tx.listen("state", function(s) tx.log(s) end)
            "#,
        ).exec().unwrap();

        assert_eq!(state.borrow().state_listeners.len(), 2);
    }

    #[test]
    fn listen_unknown_event_errors() {
        let lua = Lua::new();
        let _state = install_api(&lua).unwrap();

        let result = lua.load(
            r#"
            local tx = require("tuxinjector")
            tx.listen("banana", function() end)
            "#,
        ).exec();

        assert!(result.is_err());
    }
}
