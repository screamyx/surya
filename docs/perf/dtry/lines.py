#!/usr/bin/env python3
"""Print the self-test and pump lines of a dtry run log, unwrapped.

conhost --headless wraps the app's stdout at 120 columns and writes a cursor
move before the remainder, so a long line lands as two. This strips the
escapes and joins a line back onto a previous one that was exactly 120 wide.

    docs/perf/dtry/lines.py runs/scroll-pool.out.log
"""
import re
import sys

ESC = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")
WIDTH = 120

for path in sys.argv[1:]:
    raw = open(path, encoding="utf-8", errors="replace").read()
    joined: list[str] = []
    for line in raw.split("\n"):
        clean = ESC.sub("", line).rstrip("\r")
        if joined and len(joined[-1]) >= WIDTH and not clean.startswith(("browser:", "selftest:")):
            # conhost repeats the column-120 character after the cursor move.
            if clean and joined[-1].endswith(clean[0]):
                clean = clean[1:]
            joined[-1] += clean
        else:
            joined.append(clean)
    for line in joined:
        if line.startswith("selftest:") or re.match(r"browser: (pump|clock|frame rate|timer)", line):
            print(line)
