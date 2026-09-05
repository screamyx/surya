#!/usr/bin/env python3
"""Press, move a few pixels, then release: a click by a hand that is not still.

    x7-nudge-click.py :7 1013 55 3

A plain click never reproduced the bug it exists for. gpui turns a press plus
a few pixels of travel into a drag, so a close button inside a draggable tab
has to survive exactly this: press on the button, twitch, release. The last
argument is how far the pointer travels, in pixels, default 3.

Needs python-xlib, like x7-click.py beside it.
"""
import sys
import time

from Xlib import X, display
from Xlib.ext import xtest


def main() -> int:
    if len(sys.argv) not in (4, 5):
        print(__doc__, file=sys.stderr)
        return 2
    disp, x, y = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    travel = int(sys.argv[4]) if len(sys.argv) == 5 else 3
    d = display.Display(disp)
    if not d.query_extension("XTEST"):
        print("nudge-click: XTEST missing on", disp, file=sys.stderr)
        return 3
    xtest.fake_input(d, X.MotionNotify, x=x, y=y)
    d.sync()
    time.sleep(0.15)
    xtest.fake_input(d, X.ButtonPress, 1)
    d.sync()
    time.sleep(0.10)
    for step in range(1, travel + 1):
        xtest.fake_input(d, X.MotionNotify, x=x + step, y=y)
        d.sync()
        time.sleep(0.03)
    xtest.fake_input(d, X.ButtonRelease, 1)
    d.sync()
    print(f"nudge-click: asked=1 sent=1 at {x},{y} travel={travel}px on {disp}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
