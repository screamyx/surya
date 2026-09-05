    use super::*;
    use gpui::{Keystroke, Modifiers, MouseButton, Pixels, Point, ScrollDelta};
    use gpui::{point, px};

    fn mods() -> Modifiers {
        Modifiers::default()
    }

    fn stroke(key: &str, key_char: Option<&str>, modifiers: Modifiers) -> Keystroke {
        Keystroke {
            modifiers,
            key: key.to_string(),
            key_char: key_char.map(str::to_string),
        }
    }

    #[test]
    fn a_click_lands_where_the_page_thinks_it_did() {
        // The whole of order 4 in one assertion. Column 3 starts 226px in
        // (rail 200 plus its 1px border, plus the divider), and CEF's view
        // starts at 0. A click at window x=300 is a click at page x=74.
        assert_eq!(view_point(point(px(300.0), px(140.0)), (226, 22)), (74, 118));
    }

    #[test]
    fn a_drag_out_of_the_column_keeps_going_negative() {
        // Chromium wants the real position while a button is held, not a
        // clamped one: a text selection dragged off the left edge has to keep
        // extending. Clamping here would freeze the selection at the edge.
        assert_eq!(view_point(point(px(10.0), px(4.0)), (226, 22)), (-216, -18));
    }

    #[test]
    fn held_buttons_ride_along_on_every_mouse_event() {
        // CEF has no other way to know a button is down during a move, and a
        // page whose drag never starts is the symptom.
        let held = button_bit(MouseButton::Left).unwrap();
        assert_eq!(mouse_flags(&mods(), held), flags::LEFT_MOUSE_BUTTON);
        assert_eq!(button_bit(MouseButton::Right), Some(flags::RIGHT_MOUSE_BUTTON));
        assert_eq!(button_bit(MouseButton::Middle), Some(flags::MIDDLE_MOUSE_BUTTON));
    }

    #[test]
    fn the_three_buttons_map_to_cefs_own_numbers() {
        // `cef_mouse_button_type_t`: MBT_LEFT 0, MBT_MIDDLE 1, MBT_RIGHT 2.
        // `win.rs` converts through a match rather than a cast, so these
        // discriminants are documentation - but documentation that is wrong is
        // worse than none, and this is the only place it can be checked.
        assert_eq!(button_type(MouseButton::Left), Some(Button::Left));
        assert_eq!(button_type(MouseButton::Middle), Some(Button::Middle));
        assert_eq!(button_type(MouseButton::Right), Some(Button::Right));
        assert_eq!(Button::Left as i32, 0);
        assert_eq!(Button::Middle as i32, 1);
        assert_eq!(Button::Right as i32, 2);
    }

    #[test]
    fn the_back_button_is_not_a_cef_mouse_button() {
        use gpui::NavigationDirection;
        let back = MouseButton::Navigate(NavigationDirection::Back);
        assert_eq!(button_bit(back), None);
        assert_eq!(button_type(back), None);
    }

    #[test]
    fn keyboard_modifiers_reach_a_mouse_event() {
        let m = Modifiers { control: true, shift: true, ..Modifiers::default() };
        assert_eq!(
            mouse_flags(&m, 0),
            flags::CONTROL_DOWN | flags::SHIFT_DOWN
        );
    }

    #[test]
    fn one_wheel_notch_is_three_lines_and_one_hundred_and_twenty_units() {
        // Windows reports a notch as `wheel_scroll_lines` lines, 3 by default,
        // and the whole stack calls a notch 120. If this drifts, every page
        // scrolls the wrong distance and nothing else says so.
        let (dx, dy, f) = wheel(ScrollDelta::Lines(point(0.0, 3.0)), &mods());
        assert_eq!((dx, dy), (0, 120));
        assert_eq!(f & flags::PRECISION_SCROLLING_DELTA, 0);
    }

    #[test]
    fn a_trackpad_delta_is_marked_precise_and_passed_through() {
        let (dx, dy, f) = wheel(ScrollDelta::Pixels(point(px(0.0), px(-37.0))), &mods());
        assert_eq!((dx, dy), (0, -37));
        assert_ne!(f & flags::PRECISION_SCROLLING_DELTA, 0);
    }

    #[test]
    fn a_horizontal_wheel_does_not_also_carry_shift() {
        // GPUI already moved the delta onto x for shift+wheel. Handing
        // Chromium the shift bit as well risks a second swap.
        let m = Modifiers { shift: true, ..Modifiers::default() };
        let (dx, dy, f) = wheel(ScrollDelta::Lines(point(3.0, 0.0)), &m);
        assert_eq!((dx, dy), (120, 0));
        assert_eq!(f & flags::SHIFT_DOWN, 0);
    }

    #[test]
    fn a_vertical_wheel_with_shift_keeps_the_bit() {
        let m = Modifiers { shift: true, ..Modifiers::default() };
        let (_, _, f) = wheel(ScrollDelta::Lines(point(0.0, 3.0)), &m);
        assert_ne!(f & flags::SHIFT_DOWN, 0);
    }

    #[test]
    fn ctrl_wheel_keeps_control_so_the_page_can_zoom() {
        let m = Modifiers { control: true, ..Modifiers::default() };
        let (_, _, f) = wheel(ScrollDelta::Lines(point(0.0, -3.0)), &m);
        assert_ne!(f & flags::CONTROL_DOWN, 0);
    }

    #[test]
    fn letters_and_digits_are_their_uppercase_ascii() {
        assert_eq!(key_code("a"), Some(0x41));
        assert_eq!(key_code("z"), Some(0x5A));
        assert_eq!(key_code("0"), Some(0x30));
        assert_eq!(key_code("9"), Some(0x39));
    }

    #[test]
    fn every_name_gpui_can_produce_for_a_non_character_key_has_a_code() {
        // The list is `parse_immutable` in gpui_windows/src/events.rs. A name
        // missing here is a key that silently does nothing in the browser, and
        // the only way to notice is to press it.
        for key in [
            "space", "backspace", "enter", "tab", "up", "down", "right", "left", "home", "end",
            "pageup", "pagedown", "back", "forward", "escape", "insert", "delete", "menu",
        ] {
            assert!(key_code(key).is_some(), "no virtual key for {key}");
        }
        for n in 1..=24 {
            let key = format!("f{n}");
            assert_eq!(key_code(&key), Some(0x6F + n), "wrong code for {key}");
        }
    }

    #[test]
    fn f13_is_not_f1_followed_by_a_3() {
        assert_eq!(key_code("f1"), Some(0x70));
        assert_eq!(key_code("f13"), Some(0x7C));
        assert_eq!(key_code("f25"), None);
        assert_eq!(key_code("f0"), None);
    }

    #[test]
    fn an_unknown_key_sends_nothing_rather_than_guessing() {
        assert_eq!(key_code("capslock"), None);
        assert_eq!(key_code(""), None);
        assert_eq!(key_code("shift"), None);
    }

    #[test]
    fn punctuation_uses_the_oem_keys() {
        assert_eq!(key_code(";"), Some(0xBA));
        assert_eq!(key_code("/"), Some(0xBF));
        assert_eq!(key_code("`"), Some(0xC0));
        assert_eq!(key_code("\\"), Some(0xDC));
    }

    #[test]
    fn a_shifted_character_resolves_to_the_key_it_is_printed_on() {
        // GPUI hands us the shifted character, not the key. `@` is the 2 key.
        assert_eq!(key_code("@"), key_code("2"));
        assert_eq!(key_code("?"), key_code("/"));
        assert_eq!(key_code("{"), key_code("["));
        assert_eq!(key_code("_"), key_code("-"));
    }

    #[test]
    fn a_shifted_character_puts_the_shift_bit_back() {
        // GPUI clears `shift` when it resolves the shifted character, so the
        // page would otherwise see the 2 key pressed with no shift held.
        assert_ne!(key_flags(&mods(), "@") & flags::SHIFT_DOWN, 0);
        assert_eq!(key_flags(&mods(), "2") & flags::SHIFT_DOWN, 0);
    }

    #[test]
    fn a_capital_letter_is_not_a_shifted_character() {
        // GPUI leaves `shift` set for letters and reports the lowercase key,
        // so nothing needs putting back and nothing may be invented.
        let m = Modifiers { shift: true, ..Modifiers::default() };
        assert_ne!(key_flags(&m, "a") & flags::SHIFT_DOWN, 0);
        assert_eq!(key_flags(&mods(), "a") & flags::SHIFT_DOWN, 0);
    }

    #[test]
    fn typing_a_character_carries_the_character_gpui_resolved() {
        let ks = stroke("a", Some("a"), mods());
        assert_eq!(char_unit(&ks), Some(u16::from(b'a')));
        let m = Modifiers { shift: true, ..Modifiers::default() };
        let ks = stroke("a", Some("A"), m);
        assert_eq!(char_unit(&ks), Some(u16::from(b'A')));
    }

    #[test]
    fn the_four_control_keys_a_page_still_wants_a_character_for() {
        // GPUI filters control characters out of `key_char`, so these arrive
        // with nothing to type. A text box that ignores Enter is the symptom.
        assert_eq!(char_unit(&stroke("enter", None, mods())), Some(0x0D));
        assert_eq!(char_unit(&stroke("tab", None, mods())), Some(0x09));
        assert_eq!(char_unit(&stroke("backspace", None, mods())), Some(0x08));
        assert_eq!(char_unit(&stroke("escape", None, mods())), Some(0x1B));
    }

    #[test]
    fn a_shortcut_does_not_also_type_into_the_page() {
        // ctrl+c copies. If it types as well, the page gets a stray character
        // every time the owner copies something out of it.
        let m = Modifiers { control: true, ..Modifiers::default() };
        assert_eq!(char_unit(&stroke("c", Some("c"), m)), None);
        let m = Modifiers { platform: true, ..Modifiers::default() };
        assert_eq!(char_unit(&stroke("c", Some("c"), m)), None);
    }

    #[test]
    fn alt_still_types_because_altgr_is_alt_on_this_keyboard() {
        // AltGr arrives as alt on Windows, and it is how a non-US layout types
        // half its punctuation. Refusing a character here would make those
        // keys dead.
        let m = Modifiers { alt: true, ..Modifiers::default() };
        assert_eq!(char_unit(&stroke("e", Some("€"), m)), Some(0x20AC));
    }

    #[test]
    fn an_arrow_key_types_nothing() {
        assert_eq!(char_unit(&stroke("left", None, mods())), None);
        assert_eq!(char_unit(&stroke("f5", None, mods())), None);
    }

    // The owner's rule: column 3 behaves like a regular Chrome. An offscreen
    // browser has no browser half, so every one of these is haktui's job.

    #[test]
    fn f5_and_ctrl_r_reload() {
        let ctrl = Modifiers { control: true, ..Modifiers::default() };
        assert_eq!(shortcut(&stroke("f5", None, mods())), Some(Command::Reload));
        assert_eq!(shortcut(&stroke("r", Some("r"), ctrl)), Some(Command::Reload));
    }

    #[test]
    fn the_hard_reload_spellings_both_bypass_the_cache() {
        let ctrl_shift = Modifiers { control: true, shift: true, ..Modifiers::default() };
        let ctrl = Modifiers { control: true, ..Modifiers::default() };
        assert_eq!(
            shortcut(&stroke("r", Some("R"), ctrl_shift)),
            Some(Command::ReloadIgnoringCache)
        );
        assert_eq!(
            shortcut(&stroke("f5", None, ctrl)),
            Some(Command::ReloadIgnoringCache)
        );
    }

    #[test]
    fn alt_arrow_walks_the_history() {
        let alt = Modifiers { alt: true, ..Modifiers::default() };
        assert_eq!(shortcut(&stroke("left", None, alt)), Some(Command::Back));
        assert_eq!(shortcut(&stroke("right", None, alt)), Some(Command::Forward));
        // A plain arrow is the page's, not the chrome's.
        assert_eq!(shortcut(&stroke("left", None, mods())), None);
    }

    #[test]
    fn the_side_mouse_buttons_walk_the_history() {
        use gpui::NavigationDirection;
        assert_eq!(
            mouse_shortcut(MouseButton::Navigate(NavigationDirection::Back)),
            Some(Command::Back)
        );
        assert_eq!(
            mouse_shortcut(MouseButton::Navigate(NavigationDirection::Forward)),
            Some(Command::Forward)
        );
        assert_eq!(mouse_shortcut(MouseButton::Left), None);
    }

    #[test]
    fn both_spellings_of_ctrl_plus_zoom_in() {
        // GPUI resolves ctrl+shift+= to "+" and clears shift, so the same
        // physical chord arrives under two names.
        let ctrl = Modifiers { control: true, ..Modifiers::default() };
        assert_eq!(shortcut(&stroke("=", Some("="), ctrl)), Some(Command::ZoomIn));
        assert_eq!(shortcut(&stroke("+", Some("+"), ctrl)), Some(Command::ZoomIn));
        assert_eq!(shortcut(&stroke("-", Some("-"), ctrl)), Some(Command::ZoomOut));
        assert_eq!(shortcut(&stroke("0", Some("0"), ctrl)), Some(Command::ZoomReset));
    }

    #[test]
    fn cmd_works_as_well_as_ctrl() {
        // The owner has a Mac in the plan. A browser that ignores cmd+R there
        // is not a Chrome.
        let cmd = Modifiers { platform: true, ..Modifiers::default() };
        assert_eq!(shortcut(&stroke("r", Some("r"), cmd)), Some(Command::Reload));
    }

    #[test]
    fn the_tab_chords_are_chromes() {
        let ctrl = Modifiers { control: true, ..Modifiers::default() };
        let ctrl_shift = Modifiers { control: true, shift: true, ..Modifiers::default() };
        assert_eq!(shortcut(&stroke("t", Some("t"), ctrl)), Some(Command::NewTab));
        assert_eq!(shortcut(&stroke("w", Some("w"), ctrl)), Some(Command::CloseTab));
        assert_eq!(shortcut(&stroke("tab", None, ctrl)), Some(Command::NextTab));
        assert_eq!(shortcut(&stroke("tab", None, ctrl_shift)), Some(Command::PreviousTab));
    }

    #[test]
    fn ctrl_shift_m_is_the_device_toolbar_as_it_is_in_chrome() {
        let ctrl_shift = Modifiers { control: true, shift: true, ..Modifiers::default() };
        assert_eq!(shortcut(&stroke("m", Some("M"), ctrl_shift)), Some(Command::ToggleMobile));
        // Plain ctrl+m is nobody's. Stealing it would surprise a page that
        // binds it.
        let ctrl = Modifiers { control: true, ..Modifiers::default() };
        assert_eq!(shortcut(&stroke("m", Some("m"), ctrl)), None);
    }

    #[test]
    fn a_plain_tab_is_still_the_pages() {
        // Tab moves between form fields. Stealing it would break every login.
        assert_eq!(shortcut(&stroke("tab", None, mods())), None);
    }

    #[test]
    fn the_editing_keys_stay_the_pages() {
        // Blink implements copy, paste, select-all and undo itself once the key
        // event reaches it. Stealing them here would break them. Find is
        // not Blink's: the shell's `ctrl-f` action takes it before the page.
        let ctrl = Modifiers { control: true, ..Modifiers::default() };
        for key in ["c", "v", "x", "a", "f", "z"] {
            assert_eq!(shortcut(&stroke(key, Some(key), ctrl)), None, "stole ctrl+{key}");
        }
    }

    #[test]
    fn a_plain_letter_is_never_a_shortcut() {
        for key in ["r", "0", "-", "=", "f", "left"] {
            assert_eq!(shortcut(&stroke(key, Some(key), mods())), None);
        }
    }

    #[test]
    fn zoom_stops_where_chrome_stops() {
        // 1.2^-7.6 is 25 percent and 1.2^8.8 is 500 percent, Chrome's two ends.
        let mut level = 0.0;
        for _ in 0..100 {
            level = zoom(level, Command::ZoomIn);
        }
        assert!((1.2f64).powf(level) <= 5.01, "zoomed past 500 percent");
        for _ in 0..200 {
            level = zoom(level, Command::ZoomOut);
        }
        assert!((1.2f64).powf(level) >= 0.24, "zoomed below 25 percent");
    }

    #[test]
    fn one_zoom_step_is_about_ten_percent() {
        let scale = (1.2f64).powf(zoom(0.0, Command::ZoomIn));
        assert!((1.09..1.11).contains(&scale), "one step was {scale}");
    }

    // The Mac key: what CEF's synthetic NSEvent is built from.

    #[test]
    fn no_mac_key_event_ever_carries_an_empty_character() {
        // CEF's Mac path turns an event with both characters 0 into a
        // modifier-key event, and Chromium reads that as a keydown named
        // Unidentified whatever the type was: twelve keydowns for six keys,
        // no keyups, measured 2026-08-30 on the installed bundle.
        let names = [
            "space", "backspace", "enter", "tab", "escape", "up", "down", "left", "right", "insert",
            "delete", "home", "end", "pageup", "pagedown",
        ];
        for key in names {
            let k = mac_key(&stroke(key, None, Modifiers::default())).unwrap_or_else(|| panic!("no Mac key for {key}"));
            assert_ne!(k.character, 0, "{key} has no character");
            assert_ne!(k.unmodified, 0, "{key} has no unmodified character");
        }
        for n in 1..=20 {
            let key = format!("f{n}");
            let k = mac_key(&stroke(&key, None, Modifiers::default())).unwrap_or_else(|| panic!("no Mac key for {key}"));
            assert_eq!(k.character, 0xF704 + n - 1, "wrong NSEvent character for {key}");
        }
        for c in "abcdefghijklmnopqrstuvwxyz0123456789-=[]\\;',./`".chars() {
            let key = c.to_string();
            let k = mac_key(&stroke(&key, Some(&key), Modifiers::default())).unwrap_or_else(|| panic!("no Mac key for {key}"));
            assert_eq!(k.character, c as u16);
            assert_eq!(k.unmodified, c as u16);
        }
        let k = mac_key(&stroke("a", None, Modifiers { control: true, ..Default::default() })).unwrap();
        assert_eq!(k.character, 'a' as u16, "control+a still carries a character");
    }

    #[test]
    fn the_mac_codes_are_the_kvk_constants() {
        // From Carbon's Events.h, checked against Chromium's
        // KeyboardCodeFromKeyCode table (keyboard_code_conversion_mac.mm).
        let code = |key: &str| mac_key(&stroke(key, None, Modifiers::default())).map(|k| k.code);
        assert_eq!(code("a"), Some(0x00));
        assert_eq!(code("z"), Some(0x06));
        assert_eq!(code("q"), Some(0x0C));
        assert_eq!(code("2"), Some(0x13));
        assert_eq!(code("enter"), Some(0x24));
        assert_eq!(code("tab"), Some(0x30));
        assert_eq!(code("space"), Some(0x31));
        assert_eq!(code("backspace"), Some(0x33));
        assert_eq!(code("escape"), Some(0x35));
        assert_eq!(code("left"), Some(0x7B));
        assert_eq!(code("right"), Some(0x7C));
        assert_eq!(code("down"), Some(0x7D));
        assert_eq!(code("up"), Some(0x7E));
        assert_eq!(code("home"), Some(0x73));
        assert_eq!(code("end"), Some(0x77));
        assert_eq!(code("pageup"), Some(0x74));
        assert_eq!(code("pagedown"), Some(0x79));
        assert_eq!(code("delete"), Some(0x75));
        assert_eq!(code("f1"), Some(0x7A));
        assert_eq!(code("f5"), Some(0x60));
        assert_eq!(code("f12"), Some(0x6F));
    }

    #[test]
    fn a_shifted_character_is_the_key_it_sits_on_with_the_shifted_text() {
        // Both backends hand shift+2 over as key "@" with shift cleared.
        let k = mac_key(&stroke("@", Some("@"), Modifiers::default())).unwrap();
        assert_eq!(k.code, 0x13, "the 2 key");
        assert_eq!(k.character, '@' as u16);
        assert_eq!(k.unmodified, '2' as u16);
        // And shift+a is key "a" with shift and key_char "A".
        let k = mac_key(&stroke("a", Some("A"), Modifiers { shift: true, ..Default::default() })).unwrap();
        assert_eq!(k.code, 0x00);
        assert_eq!(k.character, 'A' as u16);
        assert_eq!(k.unmodified, 'a' as u16);
    }

    #[test]
    fn the_mac_sends_nothing_for_a_key_it_has_no_code_for() {
        assert_eq!(mac_key(&stroke("back", None, Modifiers::default())), None);
        assert_eq!(mac_key(&stroke("menu", None, Modifiers::default())), None);
        assert_eq!(mac_key(&stroke("f21", None, Modifiers::default())), None);
        assert_eq!(mac_key(&stroke("capslock", None, Modifiers::default())), None);
    }
