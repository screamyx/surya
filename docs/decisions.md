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
Owner, 2026-09-05 00:55: "i think we build using the native shadcn design, and then we can redesign by changing the tokens. motions are also first class."

Build on the stock shadcn look, unchanged.
Restyling later is a token change, never a component rewrite, so no component may carry a hard-coded colour, radius, or spacing.
Motion is first class: agent activity, cards arriving, files lighting up, panel transitions, all through one motion library, never hand-rolled.
Every screen is reviewed on phone and desktop before it merges.

## 13. Build rules

Owner, 2026-09-05 01:02: "build rule: keep every file thin, 500 loc max".

Every file 500 lines max, in the mockup and in the real app.
The mockup at `mockup/` is the ground truth for builders of the real app, stock shadcn "base-nova" preset on neutral, tokens only.

## 14. surya knows no business, workspaces bring their own cards

Owner, 2026-09-05 01:28: "this harness is used to interface with DMS and with me to develop app. i plan to use the harness like i use claude code, so it is not a specialised bespoke app for the dms."
Owner, 01:31: "we should be able to build catalog bespoke to our needs, depending on the project. e.g a kss-marketing repo might need a more data-visualizatiuon-centric and meta-specific catalog, project-jag might need catalog that shows diff, image etc."

surya knows workspaces, agents, tasks, files, previews and cards. It knows no business.
Nothing in surya's code names a car, a lead, a campaign, or the DMS. Sample data in the mockup may, code may not.

Cards come in three layers:
- surya primitives: Stack, Text, Image, Badge, Table, Chart, Diff, Form, Button, Progress. Built once, phone and desktop.
- surya built-in shapes, six generic cards: record, table, form, approval, diff-summary, metric.
- workspace cards: a `.surya/cards` folder in the repo, each card a schema plus a layout built from the primitives. Data, never code. Loaded when that workspace is open, checked against that workspace's catalog.

Agents write cards as tool calls (`show_card`), never as text in the reply. The tap on a card returns as the tool result. Reply text stays to one line when a card is shown.
Because a workspace card is JSON, an agent working in that repo can add a new card the same way it adds a skill.
Escape hatch, not day one: a built and installed surya plugin for a visual the primitives cannot express.

## 15. Rail and agent tree

Owner, 2026-09-05 01:44: "the left rails could be done better i think."
Owner, 01:46: "agents spawned by other agents are parented by their spawner."

The rail lists workspaces and agents, nothing else.
The four workspace pages, Agents, Tasks, Preview, Files, are a tab bar on the workspace page.
Workspaces collapse, only the current one is open. A closed workspace still shows its worst status as a dot and its "needs you" count.
Agents form a tree. An agent's parent is you or the agent that spawned it. Children nest under their parent in the rail and on the Agents page, folded unless one of them needs you.
Status rolls up: a child that needs you marks every ancestor's row.
Agents sort by priority: needs you, working, done, idle.
Model names stay off the rail row.
Cards and Settings sit at the bottom of the rail.

Applied to the mockup on 2026-09-05 01:50.

## 16. How the daemon drives Claude Code, probed

Probed on 2026-09-05, writeup `docs/probes/headless-controls-2026-09-05.md`.

- The daemon runs `claude -p` with stream-json in and out, `--permission-prompts host` and `--permission-prompt-tool stdio`. The second flag is what makes permission asks and AskUserQuestion arrive as `can_use_tool` control requests the GUI can answer. Without it they never come.
- Model switch mid-session works by sending `/model <name>` as a turn. The model picker sheet sends that text.
- Stop is a `control_request` `interrupt`. The running command dies within 2 seconds. Declare `perTaskStopAffordance` at initialize so an interrupt does not also kill the agent's background tasks.
- Daemon restart is `--resume <session-id>`. The session id comes from the init event and is stored per agent.
- The initialize response is the source of the slash palette and of any pending asks a reconnecting client must re-render.

Slash commands come in three classes: pass-through (most), menu with a text form (`/model`, `/effort`, the GUI opens a sheet and sends the text), and menu with no headless form (`/mcp`, `/plugin`, `/permissions`, `/config`, the GUI owns the screen over Claude Code's own files).

## 17. A fourth agent state: stopped

Ruled by the front desk on 2026-09-05 02:15 under the owner's autonomy order, while adding the seven mockup gaps.

An agent that crashed, hit a rate limit, ran out of context, or was stopped by you is `failed`, shown as "Stopped".
It counts as "needs you": it rolls up the tree like a question does, it appears in the Needs You inbox with the reason, the detail and a Retry button, and the workspace row goes red.
It is not a variant of "needs you" because the action differs: a question wants an answer, a stopped agent wants Retry or Give up.
The `failure` record carries `reason`, `detail`, `retryable`.

## 18. surya is a public project, nothing in it is specific to one person or one network

Owner, 2026-09-05 02:26: "i plan to publicize this repo one day, so it should not be too specific on my case."

- Reaching a daemon: localhost, LAN, Tailscale, or any reverse proxy. Tailscale is never required.
- Auth is the daemon's own: a pairing code shown once, exchanged for a token the app keeps. Network identity is a bonus, not the gate.
- Remote install runs over plain SSH from any daemon. Tailscale SSH is one case of it. A copy-paste installer line is always the fallback.
- Install line and update channel live on a project domain, not a company one.
- Sample data uses made-up people and hosts. One example workspace may keep a car dealer flavour so screens feel real. No real email, hostname, or agent id in the repo.
- agb, herdr, luvus, spawn-agent and the rest of the owner's fleet tooling are not dependencies and are not mentioned in product code or README.
- Before going public: a license file, and decisions.md kept with "the owner" in place of a name.

Servers, from decision talk on 02:22 to 02:25: a server is a daemon. Servers are the top of the tree, then workspaces, then agents. The server level hides when there is only one. Add server is an in-app flow: pick or type a host, checks with Fix buttons, install over SSH with the log streaming into the card, done. Adding an agent happens where you are: a plus on the workspace row, a button in the workspace tab bar, and "spawn under this agent" on an agent row.

## 19. Agent-to-agent mail is first class

Owner, 2026-09-05 02:27: "agb is first class too, agent-to-agent comms are essentials to my workflow".

surya ships its own agent mail, built into the daemon, with the concepts the owner's workflow already depends on:
- Addresses: an agent id, a workspace channel, a server channel.
- Send: a `send_message` tool every agent has, and a compose box in the GUI.
- Receive: the daemon delivers mail as the agent's next turn. No polling, no parking.
- Ack: automatic when the turn that carried the mail completes; a manual "seen" ack remains.
- Reserve on spawn: an agent spawned through surya has an address before it starts, so mail queues instead of failing.
- Liveness: the agent tree.

The owner's agb bus is one implementation of the same idea and may bridge to surya through a private adapter. It is not a dependency, per decision 18.

GUI surfaces: mail rows in the agent feed (from, delivery id, acked mark), sends as tool rows, Send message on agent rows, a Messages tab per workspace with the channel and direct threads.

## Open

None at day zero.
