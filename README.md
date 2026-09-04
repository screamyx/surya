# surya

A web harness for Claude Code.
Chat, code, a live app preview you can pin comments on, a task board, and every skill and tool Claude Code already has.
Agents keep running on the box after you close the browser.

Experimental take on a new haktui, on the web instead of a terminal.

## What it is

```
 browser (viewer)              surya daemon on the box (systemd)
 ┌──────────────────┐  AG-UI   ┌────────────────────────────────────┐
 │ code  preview    │◄────────►│ workspace                          │
 │ tasks chat       │  replay  │  ├ agents   claude CLI, stream-json │
 │ A2UI cards       │          │  ├ tasks    queue, agents pull      │
 └──────────────────┘          │  ├ preview  proxied app + pins      │
                               │  └ event log per agent             │
                               └────────────────────────────────────┘
```

## Decisions so far

See `docs/decisions.md`.

## Status

Day zero.
Nothing runs yet.
