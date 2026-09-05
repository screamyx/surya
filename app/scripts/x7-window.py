#!/usr/bin/env python3
"""Move, size and focus the app's window on a headless X display.

    x7-window.py :7 0 0 1440 900 [wait-seconds]

There is no window manager on the headless Xorg, so the app's window lands
wherever gpui asks and can run off the 1600x1000 screen. A plain
ConfigureWindow puts it where a proof shot needs it, which is what a window
manager would have done.

Picks the largest top-level window that is mapped and is not the root, which
on a display with one app is that app. Prints a counter pair.
"""
import sys
import time

from Xlib import X, display


def candidates(root):
    for window in root.query_tree().children:
        attrs = window.get_attributes()
        if attrs.map_state != X.IsViewable:
            continue
        geom = window.get_geometry()
        if geom.width < 200 or geom.height < 200:
            continue
        yield window, geom.width * geom.height


def main() -> int:
    if len(sys.argv) not in (6, 7):
        print(__doc__, file=sys.stderr)
        return 2
    d = display.Display(sys.argv[1])
    x, y, w, h = (int(v) for v in sys.argv[2:6])
    root = d.screen().root
    # A debug build under load can take a minute to map its window; waiting
    # here is cheaper than a proof run that shot an empty screen.
    deadline = time.time() + float(sys.argv[6]) if len(sys.argv) > 6 else time.time() + 120
    found = []
    while time.time() < deadline:
        found = sorted(candidates(root), key=lambda pair: pair[1], reverse=True)
        if found:
            break
        time.sleep(2)
    if not found:
        print("x7-window: asked=1 moved=0 (no mapped window)", file=sys.stderr)
        return 3
    window = found[0][0]
    window.configure(x=x, y=y, width=w, height=h)
    # There is no window manager here, so nothing gives the window the
    # keyboard. Without this every XTEST key goes to the root window and the
    # app sees a click but never a keystroke.
    window.set_input_focus(X.RevertToParent, X.CurrentTime)
    d.sync()
    time.sleep(0.3)
    geom = window.get_geometry()
    print(f"x7-window: asked=1 moved=1 now {geom.width}x{geom.height} at {geom.x},{geom.y}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
