#!/usr/bin/env python3
"""Extract measured self-test rows; retain the original and unwrapped logs.

Usage: summarize.py RUN.out.log [RUN.out.log ...]

The conhost transport repeats its last column on wrapped lines. This uses
the same observed 120-column convention as perf/dtry/lines.py. Inspect the
saved unwrapped lines alongside the originals before citing a result.
"""

import json
import pathlib
import re
import sys

ESCAPE = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")
FIELDS = re.compile(r"\b([a-z][a-z0-9_]*)=([0-9]+(?:\.[0-9]+)?)(?=\s|$)")
REQUIRED = (
    "loaded", "cef_frames", "app_frames", "surface_shown", "main_pump_us",
    "main_handoff_us", "main_surface_us", "main_ms_per_shown",
    "surface_p2d_n", "surface_p2d_p95_ms",
)


def unwrap(raw):
    joined = []
    for line in raw.split("\n"):
        clean = ESCAPE.sub("", line).rstrip("\r")
        if joined and len(joined[-1]) >= 120 and not clean.startswith(("browser:", "selftest:")):
            if clean and joined[-1].endswith(clean[0]):
                clean = clean[1:]
            joined[-1] += clean
        else:
            joined.append(clean)
    return joined


def parse_line(line):
    test = re.match(r"selftest: (ANIM|SCROLL)\b", line)
    if not test:
        return None
    pairs = FIELDS.findall(line)
    fields = {}
    for name, value in pairs:
        if name in fields:
            raise ValueError(f"duplicate metric {name}")
        fields[name] = float(value) if "." in value else int(value)
    missing = set(REQUIRED) - fields.keys()
    if missing:
        raise ValueError(f"missing metrics: {', '.join(sorted(missing))}")
    if fields["loaded"] != 1:
        raise ValueError("the self-test page was not requested successfully")
    shown = fields["surface_shown"]
    if not shown or fields["surface_p2d_n"] != shown:
        raise ValueError("fresh surface submissions and timing samples must be nonzero and equal")
    work = sum(fields[k] for k in ("main_pump_us", "main_handoff_us", "main_surface_us"))
    if abs(work / shown / 1000 - fields["main_ms_per_shown"]) > 0.000501:
        raise ValueError("main-thread average does not reconcile with measured totals")
    if test[1] == "SCROLL" and (fields.get("asked") != 60 or fields.get("sent") != 60):
        raise ValueError("the scroll workload did not send all 60 wheel events")
    return {
        "test": test[1],
        **fields,
        # A window boundary can include a prior pending frame or leave a final
        # paint pending. Do not silently call this difference exact dropped frames.
        "cef_minus_surface_window_gap": fields["cef_frames"] - shown,
        "line": line,
    }


def parse_run(path):
    lines = unwrap(path.read_text(encoding="utf-8-sig", errors="strict"))
    path.with_suffix(".unwrapped.log").write_text("\n".join(lines), encoding="utf-8")
    rows = [row for line in lines if (row := parse_line(line)) is not None]
    if sorted(row["test"] for row in rows) != ["ANIM", "SCROLL"]:
        raise ValueError("each run must contain exactly one animation and one scroll result")
    launch = path.with_name(path.name.replace(".out.log", ".launch.log"))
    close = re.findall(r"close_asked=(\d+) close_remaining=(\d+)", launch.read_text(encoding="utf-8-sig"))
    if len(close) != 1 or int(close[0][0]) < 1 or int(close[0][1]) != 0:
        raise ValueError("application close did not finish without forced termination")
    return {"source": str(path), "close_asked": int(close[0][0]), "close_remaining": 0, "rows": rows}


def main():
    if len(sys.argv) < 2:
        raise SystemExit("usage: summarize.py RUN.out.log [RUN.out.log ...]")
    results = []
    for name in sys.argv[1:]:
        try:
            results.append(parse_run(pathlib.Path(name)))
        except (OSError, UnicodeError, ValueError) as error:
            raise SystemExit(f"{name}: {error}") from error
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
