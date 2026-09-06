# surya

A native desktop app for Claude Code, with a browser, an editor, a file tree and a task board.

You sit at the app on Windows.
An engine daemon on the box you build on runs Claude Code, git, terminals and files, and streams back over your own network.
Agents keep running on the box after you close the app.

The app is Rust on GPUI.
It is a fork of [zeronsh/comet](https://github.com/zeronsh/comet), which lives in `app/` as a git subtree (decision 22).

## What it is

```
Windows machine                         Linux box
+-------------------+   ws://host:27700  +------------------------+
| surya app         | -----------------> | surya engine           |
| browser, editor,  |   + shared token   | runs Claude Code, git, |
| tree, tasks, chat | <----------------- | terminals, files       |
+-------------------+   streams back     +------------------------+
```

Windows is the product, Mac is next, Linux is a test bench (decision 28).

## Getting started

`docs/quickstart.md` has the install steps for both halves.

## Decisions so far

See `docs/decisions.md`.

## Status

In active development toward the 1.0 release candidate, which ships when it is complete (decision 30).
`docs/decisions.md` records every ruling, and the newest `docs/handoff-*.md` records where the work stands right now.

## Licence

surya is MIT licensed. See `LICENSE`.

The `app/` subtree keeps zeronsh/comet's own MIT licence in `app/LICENSE`.
Third-party notices are split in two: `app/THIRD_PARTY_NOTICES.md` covers what the built application bundles, and the root `THIRD_PARTY_NOTICES.md` covers source vendored into this repository.

## Contributing

`CONTRIBUTING.md` has the build and test commands and the pull request rules.
`AGENTS.md` is the full working guide, and every agent tool reads it through the `CLAUDE.md` shim.
To report a security issue, read `SECURITY.md` first.
