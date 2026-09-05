#!/usr/bin/env python3
"""Send keystrokes to a headless X display via XTEST.

    x7-keys.py :7 ctrl+t "example.com" enter ctrl+f "domain" enter

Each argument after the display is one step:

  * a chord, written with `+`: `ctrl+t`, `ctrl+shift+tab`, `ctrl+plus`
  * a bare key name: `enter`, `escape`, `tab`, `f5`
  * anything else is typed as literal text, one character at a time

Needs python-xlib. The browser pane's proof rig runs this with its own venv
(`SURYA_XTEST_PYTHON`), the same arrangement `x7-click.py` uses.

Prints a counter pair so a proof run can cite it: `keys: asked=N sent=N`.
"""
import sys
import time

from Xlib import X, XK, display
from Xlib.ext import xtest

MODIFIERS = {"ctrl": "Control_L", "control": "Control_L", "shift": "Shift_L", "alt": "Alt_L"}

# Key names that are not their own keysym.
NAMED = {
    "enter": "Return",
    "return": "Return",
    "escape": "Escape",
    "esc": "Escape",
    "tab": "Tab",
    "space": "space",
    "backspace": "BackSpace",
    "delete": "Delete",
    "up": "Up",
    "down": "Down",
    "left": "Left",
    "right": "Right",
    "plus": "plus",
    "minus": "minus",
    "equal": "equal",
}

# Characters whose keysym name differs from the character itself.
CHARS = {
    " ": "space",
    ".": "period",
    ",": "comma",
    "/": "slash",
    ":": "colon",
    "-": "minus",
    "+": "plus",
    "=": "equal",
    "_": "underscore",
    "?": "question",
    "'": "apostrophe",
}

SHIFTED = set(":?_+")


def code_for(d, keysym_name):
    keysym = XK.string_to_keysym(keysym_name)
    if keysym == 0:
        raise SystemExit(f"x7-keys: no keysym for {keysym_name!r}")
    code = d.keysym_to_keycode(keysym)
    if code == 0:
        raise SystemExit(f"x7-keys: no keycode for {keysym_name!r}")
    return code


def tap(d, keysym_name, mods=()):
    codes = [code_for(d, MODIFIERS.get(m, m)) for m in mods]
    for code in codes:
        xtest.fake_input(d, X.KeyPress, code)
    key = code_for(d, keysym_name)
    xtest.fake_input(d, X.KeyPress, key)
    d.sync()
    time.sleep(0.02)
    xtest.fake_input(d, X.KeyRelease, key)
    for code in reversed(codes):
        xtest.fake_input(d, X.KeyRelease, code)
    d.sync()
    time.sleep(0.06)


def type_text(d, text):
    for ch in text:
        name = CHARS.get(ch, ch)
        mods = ("shift",) if (ch.isupper() or ch in SHIFTED) else ()
        tap(d, name, mods)


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    d = display.Display(sys.argv[1])
    if not d.query_extension("XTEST"):
        print("x7-keys: XTEST missing on", sys.argv[1], file=sys.stderr)
        return 3
    asked = sent = 0
    for step in sys.argv[2:]:
        asked += 1
        lowered = step.lower()
        if "+" in step and len(step) > 1:
            parts = lowered.split("+")
            key, mods = parts[-1], parts[:-1]
            tap(d, NAMED.get(key, key), mods)
        elif lowered in NAMED:
            tap(d, NAMED[lowered])
        else:
            type_text(d, step)
        sent += 1
        time.sleep(0.25)
    print(f"keys: asked={asked} sent={sent}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
