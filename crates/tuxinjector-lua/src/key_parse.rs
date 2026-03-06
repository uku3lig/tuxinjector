// Parse key combo strings like "ctrl+F1" into GLFW keycodes.
// Case-insensitive, '+' separated. Nothing fancy.

// Turn a combo string into a sorted, deduped vec of GLFW keycodes
pub fn parse_key_combo(combo: &str) -> Result<Vec<i32>, String> {
    let mut keys = Vec::new();

    for part in combo.split('+') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        keys.push(name_to_glfw(trimmed)?);
    }

    if keys.is_empty() {
        return Err("empty key combo".into());
    }

    keys.sort();
    keys.dedup();
    Ok(keys)
}

// This is just a big lookup table. Thankfully it looks very pretty
// because i used an LLM again. and its also easy to extend if we need more keys.
//
// ── Key Name → GLFW Keycode Lookup Table ───────────────────────────────
fn name_to_glfw(name: &str) -> Result<i32, String> {
    let lower = name.to_lowercase();

    match lower.as_str() {
        // ── Alphabetic ──
        "a" => Ok(65),  "b" => Ok(66),  "c" => Ok(67),  "d" => Ok(68),
        "e" => Ok(69),  "f" => Ok(70),  "g" => Ok(71),  "h" => Ok(72),
        "i" => Ok(73),  "j" => Ok(74),  "k" => Ok(75),  "l" => Ok(76),
        "m" => Ok(77),  "n" => Ok(78),  "o" => Ok(79),  "p" => Ok(80),
        "q" => Ok(81),  "r" => Ok(82),  "s" => Ok(83),  "t" => Ok(84),
        "u" => Ok(85),  "v" => Ok(86),  "w" => Ok(87),  "x" => Ok(88),
        "y" => Ok(89),  "z" => Ok(90),

        // ── Numeric ──
        "0" => Ok(48),  "1" => Ok(49),  "2" => Ok(50),  "3" => Ok(51),
        "4" => Ok(52),  "5" => Ok(53),  "6" => Ok(54),  "7" => Ok(55),
        "8" => Ok(56),  "9" => Ok(57),

        // ── Function Keys ──
        "f1"  => Ok(290), "f2"  => Ok(291), "f3"  => Ok(292), "f4"  => Ok(293),
        "f5"  => Ok(294), "f6"  => Ok(295), "f7"  => Ok(296), "f8"  => Ok(297),
        "f9"  => Ok(298), "f10" => Ok(299), "f11" => Ok(300), "f12" => Ok(301),

        // ── Navigation & Miscellaneous ──
        "escape" | "esc" => Ok(256),
        "enter" | "return" => Ok(257),
        "tab" => Ok(258),
        "backspace" => Ok(259),
        "insert" => Ok(260),
        "delete" | "del" => Ok(261),
        "right" => Ok(262),
        "left" => Ok(263),
        "down" => Ok(264),
        "up" => Ok(265),
        "pageup" | "page_up" => Ok(266),
        "pagedown" | "page_down" => Ok(267),
        "home" => Ok(268),
        "end" => Ok(269),
        "capslock" | "caps_lock" => Ok(280),
        "scrolllock" | "scroll_lock" => Ok(281),
        "numlock" | "num_lock" => Ok(282),
        "printscreen" | "print_screen" => Ok(283),
        "pause" => Ok(284),
        "space" => Ok(32),

        // ── Modifier Keys ──
        "shift" | "lshift" | "left_shift" => Ok(340),
        "ctrl" | "control" | "lctrl" | "left_control" => Ok(341),
        "alt" | "lalt" | "left_alt" => Ok(342),
        "super" | "lsuper" | "left_super" | "meta" | "win" => Ok(343),
        "rshift" | "right_shift" => Ok(344),
        "rctrl" | "right_control" => Ok(345),
        "ralt" | "right_alt" => Ok(346),
        "rsuper" | "right_super" => Ok(347),

        // ── Punctuation & Symbols ──
        "minus" | "-" => Ok(45),
        "equal" | "equals" | "=" => Ok(61),
        "leftbracket" | "left_bracket" | "[" => Ok(91),
        "rightbracket" | "right_bracket" | "]" => Ok(93),
        "backslash" | "\\" => Ok(92),
        "semicolon" | ";" => Ok(59),
        "apostrophe" | "'" => Ok(39),
        "grave" | "grave_accent" | "`" => Ok(96),
        "comma" | "," => Ok(44),
        "period" | "." => Ok(46),
        "slash" | "/" => Ok(47),

        // ── Numpad ──
        "kp0" | "kp_0" | "numpad0" => Ok(320),
        "kp1" | "kp_1" | "numpad1" => Ok(321),
        "kp2" | "kp_2" | "numpad2" => Ok(322),
        "kp3" | "kp_3" | "numpad3" => Ok(323),
        "kp4" | "kp_4" | "numpad4" => Ok(324),
        "kp5" | "kp_5" | "numpad5" => Ok(325),
        "kp6" | "kp_6" | "numpad6" => Ok(326),
        "kp7" | "kp_7" | "numpad7" => Ok(327),
        "kp8" | "kp_8" | "numpad8" => Ok(328),
        "kp9" | "kp_9" | "numpad9" => Ok(329),
        "kp_decimal" | "numpad_decimal" => Ok(330),
        "kp_divide" | "numpad_divide" => Ok(331),
        "kp_multiply" | "numpad_multiply" => Ok(332),
        "kp_subtract" | "numpad_subtract" => Ok(333),
        "kp_add" | "numpad_add" => Ok(334),
        "kp_enter" | "numpad_enter" => Ok(335),
        "kp_equal" | "numpad_equal" => Ok(336),

        _ => Err(format!("unknown key name: '{name}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_key() {
        assert_eq!(parse_key_combo("F1").unwrap(), vec![290]);
        assert_eq!(parse_key_combo("f1").unwrap(), vec![290]);
        assert_eq!(parse_key_combo("A").unwrap(), vec![65]);
    }

    #[test]
    fn modifier_combo() {
        let keys = parse_key_combo("ctrl+F1").unwrap();
        assert_eq!(keys, vec![290, 341]); // sorted: F1=290, ctrl=341
    }

    #[test]
    fn multi_modifier() {
        let keys = parse_key_combo("ctrl+shift+Z").unwrap();
        assert!(keys.contains(&90));  // Z
        assert!(keys.contains(&340)); // shift
        assert!(keys.contains(&341)); // ctrl
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(
            parse_key_combo("Ctrl+Shift+F1").unwrap(),
            parse_key_combo("ctrl+shift+f1").unwrap()
        );
    }

    #[test]
    fn unknown_key_error() {
        assert!(parse_key_combo("ctrl+banana").is_err());
    }

    #[test]
    fn empty_string_error() {
        assert!(parse_key_combo("").is_err());
    }

    #[test]
    fn special_keys() {
        assert_eq!(parse_key_combo("escape").unwrap(), vec![256]);
        assert_eq!(parse_key_combo("space").unwrap(), vec![32]);
        assert_eq!(parse_key_combo("enter").unwrap(), vec![257]);
    }

    #[test]
    fn aliases() {
        assert_eq!(
            parse_key_combo("esc").unwrap(),
            parse_key_combo("escape").unwrap()
        );
        assert_eq!(
            parse_key_combo("del").unwrap(),
            parse_key_combo("delete").unwrap()
        );
        assert_eq!(
            parse_key_combo("return").unwrap(),
            parse_key_combo("enter").unwrap()
        );
    }

    #[test]
    fn deduplicates() {
        let keys = parse_key_combo("ctrl+ctrl+F1").unwrap();
        assert_eq!(keys, vec![290, 341]); // deduplicated
    }
}
