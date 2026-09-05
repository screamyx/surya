#!/usr/bin/env python3
"""Click once at an absolute root position on a headless X display via XTEST.

    x7-click.py :7 129 69

Needs python-xlib (a uv venv with it is fine: run this file with that venv's
python). The shoot rigs use it for the one click a frame needs, e.g. the
titlebar globe that opens the Browser surface on builds without a launch knob.
"""
import sys
import time

from Xlib import X, display
from Xlib.ext import xtest


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__, file=sys.stderr)
        return 2
    disp, x, y = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    d = display.Display(disp)
    if not d.query_extension("XTEST"):
        print("click: XTEST missing on", disp, file=sys.stderr)
        return 3
    xtest.fake_input(d, X.MotionNotify, x=x, y=y)
    d.sync()
    time.sleep(0.15)
    xtest.fake_input(d, X.ButtonPress, 1)
    d.sync()
    time.sleep(0.08)
    xtest.fake_input(d, X.ButtonRelease, 1)
    d.sync()
    print(f"click: asked=1 sent=1 at {x},{y} on {disp}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
