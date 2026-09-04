# Decisions

Brainstorm on 2026-09-05, owner and jag-0905-raven.
Each entry is a decision, the reason, and where the fact came from.

## 1. Claude Code plugs in as a CLI wrap, through the Agent SDK

The Agent SDK is itself a CLI wrap.
Its package manifest pins a Claude Code binary, read from `@anthropic-ai/claude-agent-sdk@0.3.260` manifest.json: `"version": "2.1.260"`.
Its option doc reads: "Path to the Claude Code executable. Uses the built-in executable if not specified."

Surya uses the SDK adapter with `pathToClaudeCodeExecutable` pointed at the `claude` on PATH.
That keeps typed events and the AG-UI adapter, and the binary is whatever was last updated.
Fallback if the SDK parser lags the CLI: talk `claude -p --input-format stream-json --output-format stream-json` directly.

## 2. Auth is OAuth, never an API key

`claude auth status` on the box reports `"authMethod": "claude.ai"`, `"subscriptionType": "max"`.
The SDK reads `CLAUDE_CODE_OAUTH_TOKEN` from env, found in its source.
`claude setup-token` mints that token: "Set up a long-lived authentication token (requires Claude subscription)".
The daemon runs as the logged-in user or carries that one env var.

## 3. Nothing is stripped from Claude Code

Skills, hooks, MCP, subagents, plugins and settings all load as in the terminal.
PR 568 in project-jag set `tools: []` and `settingSources: []`; that is the anti-pattern.

Probed on 2026-09-05 with Claude Code 2.1.260 in print mode:
- Slash commands work: prompt `/probe-skill` returned the skill's canary text, and the init event listed the skill under `slash_commands`.
- Subagent output streams and is attributable: the subagent's events carried `"parent_tool_use_id"` set to the parent's Agent call id, top-level events carry none.

## 4. Transport is AG-UI, agent-drawn UI is A2UI

A2UI README: "Agents send a declarative JSON format describing the intent of the UI. The client application then renders this using its own native component library."
Status line from the same README: "current production release is v0.9.1", "Expect changes".
The React renderer is listed "Stable" for v0.9.1 on the renderers page.

Two kinds of UI, kept apart:
- Panels (diff, file tree, terminal, task board) are fixed components surya builds.
- Agent answers go through A2UI from a catalog surya registers.
- The agent never emits raw HTML.

CopilotKit is the candidate chat shell, not a commitment.

## 5. Surya abilities reach the agent as MCP servers

Preview and tasks become MCP servers surya hands to Claude Code through its normal config.
No fork of Claude Code, so version independence holds.

## 6. Preview is one shared view

The app sits behind a proxy that injects a bridge script.
The user drives the iframe, the agent drives the same iframe through the preview MCP.
Pins are messages with a selector and a crop.
project-jag `tools/lavish-live` already does the proxy and pin overlay; reuse it.

## 7. Persistence comes from the daemon, not from luvus

The daemon is a systemd unit.
A `claude` started with `--input-format stream-json` stays alive across turns while the daemon holds its stdin.
Each agent has an event log on disk; the browser replays it on reconnect.
Claude Code `--resume <session-id>` covers a daemon crash.

## 8. Workspaces are built in, luvus is not a dependency

Luvus groups agents by workspace and stays alive on disconnect.
Surya copies the data model, not the program:

```
 workspace   repo path + branch + worktree
   agents    one claude process each, own event log
   tasks     queue with deps and status, agents pull
   preview   proxied app URL for this checkout
```

Skipped in v1: file leases, merge train, gates.
Luvus `task` command set is the vocabulary to copy: add, list, claim, next, start, done, release.

## 9. Runs already outlive the browser, but the runtime cannot cancel them

Probed on 2026-09-05 against `@copilotkit/runtime` 1.69.2 with a fake agent and the real Claude adapter, with two controls.
Full writeup: `docs/probes/agui-disconnect-2026-09-05.md`.

Finding, quoted from that writeup: "when the client disconnects the runtime only unsubscribes the HTTP reader from a hot ReplaySubject, while the agent itself runs on to RUN_FINISHED in a fire-and-forget async function, so nothing is aborted and every event after the disconnect is lost rather than cancelled."

Numbers: fake agent emitted=44, client saw 6 before the kill, logged_after_disconnect=38.
Real Claude adapter: `claude` child alive 4s after the disconnect, run reached RUN_FINISHED 34s later.
Control with the client connected: 40 of 40 ticks delivered.

Three consequences for the daemon:
- Reconnect through the runtime's connect endpoint, not a fresh run, and keep our own event log on disk, since the runtime drops post-disconnect events at its write guard.
- The runtime's stop endpoint does not stop Claude. `abortRun()` is empty in the base agent and the Claude adapter never overrides it. The daemon owns the `claude` pid, or subclasses `abortRun()` to call the adapter's `interrupt()`.
- A second run on a busy thread calls the same empty abort, so two `claude` processes result. The daemon must serialise runs per agent itself.
- Replace the in-memory runner if runs must survive a daemon restart.

## 10. Runtime is Bun

Probed on 2026-09-05 with Bun 1.3.14 on the box.
The real Claude adapter server from the disconnect probe started under `bun run` and listened in 14 ms.
One run through it, with a Bash tool call, returned the canary `BUN_PROBE_OK_42` and reached RUN_FINISHED, server errors=0.
The Agent SDK README documents Bun outright, including `bun build --compile` and an `extractFromBunfs` helper for the bundled binary.
Node stays as the fallback, nothing in the stack is Bun-only.

## 11. The 1.0 promise and what ships first class

Owner, 2026-09-05 00:51, on A2UI, file tree and editor: "must ship as first class on the get go. these are what seperate us from the other generic harness."

The 1.0 promise: you can start work from your phone, leave, and come back to either a finished result or one clear question waiting for you.

Ships first class in 1.0:
1. Ask in a sentence: pick a workspace, type the ask, an agent starts.
2. Needs You inbox with push: every permission ask and agent question in one list, tap to answer.
3. Plain status per agent: working, needs you, done, plus a one-line human summary.
4. Preview with pins: tap the app, type a note, the pin becomes a task.
5. Result you can act on: plain-words summary, diff, Ship button through PR to merged.
6. A2UI cards: agent answers rendered from a surya catalog, not raw HTML.
7. File tree and editor: browse the workspace, open and edit files, see the agent's edits land live.

Invisible but required: sessions never lost, task board per workspace, a stop that kills the process, phone first on the tailnet.

Cut from 1.0: terminal panel, luvus module, leases, merge train, multi-user accounts.

Release candidate bar: the owner runs a real project-jag task end to end from a phone, no terminal, no developer beside him, and the result merges.

## 12. Exceptional GUI, whole app in React on shadcn/ui

Owner, 2026-09-05 00:53: "people today are superficial (like me), form what attract attentions over feature. since a2ui is already built on react, the whole app should be built on react. that way we can start building with component libraries like shadcnui or equivalents."

Ships first class in 1.0 as feature 8: the look.
The whole app is React.
Components come from shadcn/ui on Tailwind, or an equivalent, never hand-rolled.
The A2UI catalog is built from the same shadcn components, so agent cards and surya panels are one design system.
A design direction is chosen and written down before the first screen is built, and every screen is reviewed against it.

## Open

None at day zero.
