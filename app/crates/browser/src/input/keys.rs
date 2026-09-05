//! Virtual key codes, key flags and the typed character.

use gpui::{Keystroke, Modifiers};

use super::flags;
use super::mouse::modifier_flags;

/// Modifier bits for a key event.
///
/// GPUI's Windows backend resolves a shifted punctuation key to the shifted
/// character and then **clears** `shift`: `shift+2` arrives as `key == "@"`
/// with `modifiers.shift == false`. Chromium is about to be handed the virtual
/// key for `2`, so without putting the bit back the page would see `2` pressed
/// with no shift and `event.key` would be wrong on every keydown listener.
pub fn key_flags(m: &Modifiers, key: &str) -> u32 {
    let mut f = modifier_flags(m);
    if key.chars().count() == 1 && us_shifted_base(key.chars().next().unwrap()).is_some() {
        f |= flags::SHIFT_DOWN;
    }
    f
}

/// The Windows virtual key code for a GPUI key name.
///
/// `None` means "no key event for this", which is the right answer for a key
/// this build has never seen rather than a guess that types something.
pub fn key_code(key: &str) -> Option<i32> {
    if let Some(vk) = named_key(key) {
        return Some(vk);
    }
    let mut chars = key.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    char_key(c)
}

fn named_key(key: &str) -> Option<i32> {
    // Every name GPUI's Windows backend can produce for a key that is not a
    // character. Read from `parse_immutable` in gpui_windows/src/events.rs, not
    // from memory.
    let vk = match key {
        "space" => 0x20,
        "backspace" => 0x08,
        "enter" => 0x0D,
        "tab" => 0x09,
        "up" => 0x26,
        "down" => 0x28,
        "right" => 0x27,
        "left" => 0x25,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "back" => 0xA6,
        "forward" => 0xA7,
        "escape" => 0x1B,
        "insert" => 0x2D,
        "delete" => 0x2E,
        "menu" => 0x5D,
        _ => return f_key(key),
    };
    Some(vk)
}

fn f_key(key: &str) -> Option<i32> {
    let n: u32 = key.strip_prefix('f')?.parse().ok()?;
    // VK_F1 is 0x70 and they run consecutively to VK_F24.
    (1..=24).contains(&n).then(|| 0x6F + n as i32)
}

fn char_key(c: char) -> Option<i32> {
    if c.is_ascii_alphabetic() {
        // VK_A .. VK_Z are the uppercase ASCII codes.
        return Some(c.to_ascii_uppercase() as i32);
    }
    if c.is_ascii_digit() {
        return Some(c as i32);
    }
    let base = us_shifted_base(c).unwrap_or(c);
    if base.is_ascii_digit() {
        return Some(base as i32);
    }
    oem_key(base)
}

/// The OEM virtual keys, which are the ones a US layout puts punctuation on.
///
/// This is a **US-layout assumption** and the one place in this file that is.
/// On another layout the character reaches the page correctly through the CHAR
/// event, which carries the character itself; only the virtual key on the
/// keydown would be wrong, so a shortcut bound to a punctuation key could miss.
fn oem_key(c: char) -> Option<i32> {
    let vk = match c {
        ';' => 0xBA, // VK_OEM_1
        '=' => 0xBB, // VK_OEM_PLUS
        ',' => 0xBC, // VK_OEM_COMMA
        '-' => 0xBD, // VK_OEM_MINUS
        '.' => 0xBE, // VK_OEM_PERIOD
        '/' => 0xBF, // VK_OEM_2
        '`' => 0xC0, // VK_OEM_3
        '[' => 0xDB, // VK_OEM_4
        '\\' => 0xDC, // VK_OEM_5
        ']' => 0xDD, // VK_OEM_6
        '\'' => 0xDE, // VK_OEM_7
        _ => return None,
    };
    Some(vk)
}

/// The unshifted character a US layout puts on the same key.
fn us_shifted_base(c: char) -> Option<char> {
    let base = match c {
        '~' => '`',
        '!' => '1',
        '@' => '2',
        '#' => '3',
        '$' => '4',
        '%' => '5',
        '^' => '6',
        '&' => '7',
        '*' => '8',
        '(' => '9',
        ')' => '0',
        '_' => '-',
        '+' => '=',
        '{' => '[',
        '}' => ']',
        '|' => '\\',
        ':' => ';',
        '"' => '\'',
        '<' => ',',
        '>' => '.',
        '?' => '/',
        _ => return None,
    };
    Some(base)
}

/// The UTF-16 unit for CEF's CHAR event, which is what actually types text.
///
/// `key_char` is GPUI's answer and it is the right one: it comes from
/// `ToUnicode` against the live keyboard state, so it already respects the
/// layout, AltGr and dead keys. It is `None` for control characters, so the
/// four keys a page still expects a CHAR for are listed here.
///
/// **No CHAR while control or the platform key is held.** `ctrl+c` is a
/// shortcut; sending a character alongside it types into the page as well as
/// copying from it.
pub fn char_unit(ks: &Keystroke) -> Option<u16> {
    if ks.modifiers.control || ks.modifiers.platform {
        return None;
    }
    if let Some(text) = ks.key_char.as_ref()
        && let Some(unit) = text.encode_utf16().next()
    {
        return Some(unit);
    }
    let c = match ks.key.as_str() {
        "enter" => '\r',
        "tab" => '\t',
        "backspace" => '\u{8}',
        "escape" => '\u{1b}',
        _ => return None,
    };
    Some(c as u16)
}

/// What CEF's Mac path needs on every key event.
///
/// On the Mac CEF does not read `windows_key_code` at all. It builds a
/// synthetic `NSEvent` from `native_key_code` (the Mac's own `kVK_*` code) and
/// the two character fields, and Chromium derives `windows_key_code`, `code`
/// and `key` from that event (`browser_platform_delegate_native_mac.mm`,
/// `TranslateWebKeyEvent`, read raw for CEF 151). Two consequences that were
/// measured as bugs on the installed bundle (2026-08-30, KEYS self-test):
///
/// - An event whose `character` and `unmodified_character` are both 0 is
///   turned into `NSEventTypeFlagsChanged`, a modifier-key event. Chromium
///   then decides down or up from the modifier state, not from the type, and
///   for a non-modifier key code that reads as "down". So a RAWKEYDOWN with no
///   character was a keydown named `Unidentified`, and the KEYUP with no
///   character was a **second** keydown named `Unidentified`: twelve keydowns
///   for six keys, no keyups, and the arrows and backspace dead in the page.
/// - `native_key_code` 0 is `kVK_ANSI_A`, so every key reported `code=KeyA`.
///
/// So on the Mac every event carries the key's `kVK` code and the character
/// its real `NSEvent` would: the Unicode function-key range (`0xF700..`) for
/// the arrows, paging and F keys, the control characters for enter, tab,
/// backspace and escape, and the typed character otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MacKey {
    /// `kVK_*`, the value Chromium reads `event.code` from.
    pub code: u16,
    /// What `NSEvent.characters` holds: never 0.
    pub character: u16,
    /// What `NSEvent.charactersIgnoringModifiers` holds: never 0.
    pub unmodified: u16,
}

/// The Mac key for a GPUI keystroke. `None` for a key the Mac has no code
/// for, which sends nothing rather than a guess.
pub fn mac_key(ks: &Keystroke) -> Option<MacKey> {
    if let Some((code, character)) = mac_named_key(&ks.key) {
        return Some(MacKey { code, character, unmodified: character });
    }
    let mut chars = ks.key.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let base = us_shifted_base(c).unwrap_or(c);
    let code = mac_char_key(base)?;
    // The typed character when GPUI knows it (`ToUnicode`'s answer on the live
    // layout: shift, option and dead keys applied), else the key itself. Under
    // control or command GPUI gives no `key_char`; the key's own character is
    // what a real NSEvent carries there too.
    let character = ks
        .key_char
        .as_ref()
        .and_then(|t| t.encode_utf16().next())
        .unwrap_or(c as u16);
    Some(MacKey { code, character, unmodified: base as u16 })
}

/// `(kVK, NSEvent character)` for every non-character key name gpui_macos
/// produces (`parse_keystroke` in gpui_macos/src/events.rs). The character is
/// the one AppKit puts in `characters` for that key: the `NS*FunctionKey`
/// constants for the function keys, the ASCII control code otherwise.
fn mac_named_key(key: &str) -> Option<(u16, u16)> {
    let pair = match key {
        "space" => (0x31, 0x20),
        "backspace" => (0x33, 0x7F),
        "enter" => (0x24, 0x0D),
        "tab" => (0x30, 0x09),
        "escape" => (0x35, 0x1B),
        "up" => (0x7E, 0xF700),
        "down" => (0x7D, 0xF701),
        "left" => (0x7B, 0xF702),
        "right" => (0x7C, 0xF703),
        "insert" => (0x72, 0xF727),  // Help, which Chromium reads as Insert
        "delete" => (0x75, 0xF728),  // forward delete
        "home" => (0x73, 0xF729),
        "end" => (0x77, 0xF72B),
        "pageup" => (0x74, 0xF72C),
        "pagedown" => (0x79, 0xF72D),
        _ => return mac_f_key(key),
    };
    Some(pair)
}

fn mac_f_key(key: &str) -> Option<(u16, u16)> {
    let n: u16 = key.strip_prefix('f')?.parse().ok()?;
    // The F keys have no order on the Mac keyboard: `kVK_F1` is 0x7A and
    // `kVK_F5` is 0x60. Their characters do run: `NSF1FunctionKey` is 0xF704.
    let code = match n {
        1 => 0x7A,
        2 => 0x78,
        3 => 0x63,
        4 => 0x76,
        5 => 0x60,
        6 => 0x61,
        7 => 0x62,
        8 => 0x64,
        9 => 0x65,
        10 => 0x6D,
        11 => 0x67,
        12 => 0x6F,
        13 => 0x69,
        14 => 0x6B,
        15 => 0x71,
        16 => 0x6A,
        17 => 0x40,
        18 => 0x4F,
        19 => 0x50,
        20 => 0x5A,
        _ => return None,
    };
    Some((code, 0xF704 + (n - 1)))
}

/// `kVK_ANSI_*` for the character a US layout puts on the key. The same
/// US-layout assumption as `oem_key`, with the same consequence: on another
/// layout the text still types (the characters carry it) and only `code` on
/// the keydown is that of the US key.
fn mac_char_key(c: char) -> Option<u16> {
    let code = match c.to_ascii_lowercase() {
        'a' => 0x00,
        's' => 0x01,
        'd' => 0x02,
        'f' => 0x03,
        'h' => 0x04,
        'g' => 0x05,
        'z' => 0x06,
        'x' => 0x07,
        'c' => 0x08,
        'v' => 0x09,
        'b' => 0x0B,
        'q' => 0x0C,
        'w' => 0x0D,
        'e' => 0x0E,
        'r' => 0x0F,
        'y' => 0x10,
        't' => 0x11,
        '1' => 0x12,
        '2' => 0x13,
        '3' => 0x14,
        '4' => 0x15,
        '6' => 0x16,
        '5' => 0x17,
        '=' => 0x18,
        '9' => 0x19,
        '7' => 0x1A,
        '-' => 0x1B,
        '8' => 0x1C,
        '0' => 0x1D,
        ']' => 0x1E,
        'o' => 0x1F,
        'u' => 0x20,
        '[' => 0x21,
        'i' => 0x22,
        'p' => 0x23,
        'l' => 0x25,
        'j' => 0x26,
        '\'' => 0x27,
        'k' => 0x28,
        ';' => 0x29,
        '\\' => 0x2A,
        ',' => 0x2B,
        '/' => 0x2C,
        'n' => 0x2D,
        'm' => 0x2E,
        '.' => 0x2F,
        '`' => 0x32,
        _ => return None,
    };
    Some(code)
}
