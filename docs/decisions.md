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

SUPERSEDED IN PART by decision 22 (2026-09-05): the transport is comet's engine event stream over its typed RPC, not AG-UI, and the A2UI renderer is native GPUI (`app/crates/a2ui`), not the React renderer. A2UI as the agent-drawn UI format stands.

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

SUPERSEDED by decision 22 (2026-09-05): the app is native GPUI on comet; the React mockup under `mockup/` is frozen as the design reference. "Exceptional GUI" stands as the bar (decision 20 and the critique rounds carry it).

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

Shown cards are recorded one file per chat, at `~/.surya/cards/<chat id>.jsonl` - the same root the mail ingress resolves, not the headless engine's data dir - and the engine hands that path to the agent on every real run (2026-09-06). There was no prior convention - the app reads whichever path the run carries - so this is the choice, written down rather than left implicit.

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

## 19. Agent mail is first class, and thin

Owner, 2026-09-05 02:27: "agb is first class too, agent-to-agent comms are essentials to my workflow".
Owner, 02:28: "the comm tools will mimic agb - no agent-to-agent prompt (only as fallback), and use native claude/codex messaging tool."
Owner, 02:35, after weighing a full port of agb: "thin it is, and cross-server mail is day one".

surya is the host of every agent, so mail needs no broker, no wake service, no pane binding.
The daemon keeps one table of messages and delivers a message by injecting it into the recipient's next turn: queued while a turn runs, delivered the moment it ends, all pending mail in order as one turn.
Prompting an agent's pane is the fallback only.

What stays from agb, because the owner's briefs and skills depend on it:
- Addresses: an agent id, `#workspace`, `#server`.
- A delivery id on every message. Ack is automatic when the carrying turn completes; a manual "seen" ack remains.
- Reserve on spawn: the address exists the moment the agent is created, so mail queues instead of failing.
- The words: a thin `agb` command that calls the daemon over its local socket keeps `agb send`, `agb drain`, `agb ack` working in every existing skill.

Cross-server mail is day one: an agent on one server messages an agent on another and the daemons forward over the same authenticated HTTP the app uses.

Size: one table, one `send_message` MCP tool, one delivery rule in the run loop, one CLI shim, one forwarding call, one screen.
agb's own repo may go public; surya does not depend on it.

## 20. The flows change with the research, and the look stays warm

Owner, 02:52: "im still not satisfied with the mockup. it looks too ai-generated."
Owner, 03:01: "not only the UI, the UX needs massive improvement. now im thinking for him to send researchers to scout other ADEs and find out how they do it."
Owner, 03:58, picking from the coordinator's options: "All three flows + phone cut" and "Keep warm workshop".

Grounding: `docs/research/synthesis.md`, written from twelve products and 55 screenshots (`docs/research/*.md`).
Its headline: "Eight of the ten products have no 'needs you' queue at all."
The inbox is the one idea the field lacks, so it becomes the spine of the app.

What changes in the mockup, and then in the product:
1. Home is the attention queue: what needs you at full weight, then what is running, then what is done and unshipped. Workspace and server are labels and filters, not the top grouping. The quiet state says nothing needs attention.
2. The agent session is one surface with panes: transcript, changed files, diff, live preview. Feed, Files and Preview stop being separate top-level routes. Result stays a distinct surface for the ship decision only.
3. A permission answer can become a rule. Every permission card carries a second, quieter action of the shape "Always allow migrations in project-jag", and Settings gains the approval-policy page those rules live on.
4. The phone cuts rather than shrinks. Supervising, answering and reviewing stay. The editor and the file tree are desktop only.

The look: the warm-workshop direction from `docs/design-brief.md` stays. Parchment canvas, warm neutrals, one terracotta accent, serif page titles, anchored on the kit's Claude design system file. Every new layout is designed to it.

Decisions 11 (the seven 1.0 features) and 15 (rail and agent tree) still hold; this decision changes where the features sit, not whether they ship.

## 21. The preview has two engines, and a third-party site is non-negotiable

Owner, 04:58: "a browsable third party site is non-negotiable bro. just use something like ocic to connect to the iframe or something".

Decision 6 stands for the app you are building: proxy plus iframe, bridge script, DOM pins, native scroll.
It cannot show a third-party site: most sites forbid framing by header, and a cross-origin frame is sealed against pins and element picking. Proxying a third-party site to strip that breaks logins and is not pursued.

The second engine is a real Chrome owned by the daemon, painted into the pane as a frame stream over the Chrome DevTools Protocol, with mouse, keyboard and scroll forwarded back.
- Any URL loads. The profile persists on the daemon, so the user signs in once, from any device, phone included.
- The agent drives the same Chrome over the same protocol, so decision 6's "one view, two drivers" holds for both engines.
- Pins still resolve to a selector: the daemon asks Chrome which element is under the click, and stores selector plus crop as before.
- open-claude-in-chrome is a connection option inside this engine, "use my desktop Chrome", for sites only the user's own signed-in profile can reach; it needs that desktop awake and cannot paint the tab on a phone, so it is never the default.

The pane shows which engine is active. App is the default when the workspace has a dev server; Browser is one tap away and remembers its last URL.

## 22. surya is a fork of comet, native on GPUI, with haktui's browser, editor and tree

Owner, 05:31: "stop surya-unslop. im heading to a new design direction".
Owner, 05:35, three sketches (rail card, main feed with A2UI, floating composer; a right pane with a URL bar for browser or editor; a floating file browser over it) with "inspiration: raycast, craft, cleanshot" and the gpui-kit docs.
Owner, 05:37, on github.com/zeronsh/comet: "bro's done like 80 of the work".
Owner, 05:40: "haktui has its own editor and tree, its good enough. go on the fork."

What this replaces:
- Decision 1 (Agent SDK wrap) and decision 10 (Bun daemon): comet's engine is a Rust daemon that runs Claude Code over stream-json, headed or headless, one binary. The Agent SDK is not used. Bun stays only for tooling.
- Decision 12 (React on shadcn): the app is native GPUI. The twelve-surface React mockup under `mockup/` is frozen as a design reference and the record of the flows; it does not ship.
- Decision 21's engines: the preview is haktui's CEF module composited into the GPUI window, any site, agents over CDP, pins from haktui. No proxy, no extension, no streaming.

What stays: every product rule in decisions 4 to 9, 11, 13 to 20 (A2UI cards, MCP tools, shared preview, persistence, workspaces, the seven 1.0 features, thin files, no business in surya, rail and agent tree, headless controls, stopped state, public repo and servers, thin mail, the research flows).

Layout, from the sketches: floating rounded panels on a soft canvas. Rail card, main feed, composer pill, right pane for browser or editor with a URL bar, file browser as a floating card over it. Three states: chat alone, chat plus right pane, chat plus right pane plus files. Light first like Craft, dark as good as Raycast, following the system.

Sources: `app/` is zeronsh/comet v0.2.34 (fe35546), MIT, added as a git subtree so upstream can be pulled. Browser, editor and file tree come from `~/git/haktui` (`crates/haktui/src/browser/`, `haktui_files`, the editor column), which already build on this box and on the owner's Mac.
Comet pins wingleeio's gpui fork for glass effects; haktui pins Zed's. Making haktui's modules build on comet's fork is the first engineering risk and the first task.

## 23. The rename lands last, copies the old data dir, and skips iOS

Coordinator ruling, 2026-09-05 08:20, owner asleep (autonomy order 06:17).
The plan and the counts are in `docs/rename-surya-to-surya.md` (PR #16): "surya files=260 hits=2140", "SURYA_* ... 66 distinct vars, 16 user-set need the alias".

- The rename is the last code change before the RC, after the last feature PR, as four commits in the order the plan gives.
- Existing data: on first start, if the surya data dir is absent and the surya one exists, surya COPIES it and leaves the old dir untouched. Never rename or delete a user's directory. One release later the copy step can go.
- Every user-set `SURYA_*` variable keeps working for one release as an alias of its `SURYA_*` name; the engine logs one line when the old name is used.
- iOS bundle id and the Cloudflare edge are out of scope for the RC; they keep the surya names until the owner picks a domain.
- Amended 11:03 (PR #48): the data-dir adoption is one helper `adopt(home, current, previous)` that COPIES into `<current>.incoming` and renames it in atomically; the pre-existing `.comet-native` migration now goes through it too, so it copies where it used to rename (accepted, safer, one path). After the rename the pair is (`.surya`, `.surya`) and the `.comet-native` step drops.
- Amended 10:33 from the rename dry run (PR #41): the data dir is COPIED, not renamed (this decision wins over row 5 of the plan). Comet's two built-in themes become `comet_light` / `comet_dark` and keep their display names ("Surya Light/Dark" stays as the provenance label); `surya_light` / `surya_dark` already exist from PR #2, so a literal rename would collide (E0428).
- Amended 20:20 by the comet-look revert (PR #82). Those are the Rust *function* names; the variant ids in the registry are the strings `surya-light` and `surya-dark`, and they are what the code and the settings file carry. Two things follow for whoever does the rename. Comet's pair is the shipped default again, so `ThemeSelection::default()`, the `ThemeRegistry::resolve` fallback and `Theme::for_appearance`'s fallback all name those strings and must move with them. And every `ui-settings.json` on disk stores the selection as a string, so renaming the ids without a settings migration silently drops users onto the fallback. `UiSettings::migrated` now has a schema version and one rule; the rename adds the second.

## 24. CI runs on a self-hosted runner on pc-ajim; the owner's spawn order narrows to user2

Coordinator ruling, 2026-09-05 12:49, owner asleep (autonomy order 06:17).

GitHub-hosted jobs stopped starting at 12:35 local. Every run since, main pushes and PR runs alike, fails in 3-5 s with 0 steps. The check-run annotation, read raw off the API by raven, says verbatim: "The job was not started because recent account payments have failed or your spending limit needs to be increased. Please check the 'Billing & plans' section in your settings". Fixing billing is spend and stays the owner's call.

- CI moves to a self-hosted runner on this box: `pc-ajim-surya`, label `surya-ci`, service `actions.runner.screamyx-surya.pc-ajim-surya`, dir `/store/gha-runner-surya`, running as `user` like the project-jag, haktui and code-search runners (PR #70).
- The service is throttled (systemd drop-in: CPUWeight=20, MemoryHigh=24G) so the owner's interactive session wins under load, and it builds into a persistent `CARGO_TARGET_DIR=/store/surya-ci-target` set in the runner's `.env`. That directory is the cache; rust-cache is gone.
- The hosted-only steps are gone with it: `sudo rm -rf` of SDK directories, `apt-get`, rust-cache. CI never runs `sudo` on a shared machine.
- This supersedes the ci.yml header's "GitHub-hosted on purpose" (PR #37 era). If the owner restores billing, moving back is one `runs-on` edit plus restoring those steps.
- Owner order 12:48, verbatim: "no more spawn on user's side, only on this side of luvus". No new seats on the `user` uid from now; the live user seats run to their retirement and are not respawned.

## 25. The Windows RC ships with the browser pane

Owner ruling, 2026-09-05 13:10, verbatim: "wait no. windows version with cef must ship tomorrow".

Context: raven told the owner the Windows zip does not carry the browser (build.ps1 never enables the `browser` feature, the CEF runtime is not packaged) and proposed post-RC. Overruled.

- The RC2 zip for FINAL MAIN (2026-09-06 12:15) is built with `--features browser` and ships the CEF runtime next to `surya.exe`; a real page must paint in the pane on dtry before the zip is called final.
- Path: the OSR CPU-upload path (the one proven on Linux) first; D3D11 zero-copy only if it comes free.
- Owners: surya-cef2 (crate, CEF packaging step in build.ps1, dtry proof), surya-remote (rest of build.ps1, boot-dial timeout, cmd quoting, final zip). One builder on dtry at a time, agreed over agb.
- The cut order in the 12:44 handoff is amended: the browser pane on Windows is no longer cuttable. If it is not painting by 2026-09-06 09:00, raven escalates to the owner instead of cutting.

## 26. The look goes back to comet's; the features stay

Owner ruling, 2026-09-05 19:25, verbatim: "just revert back the gui to how surya's comet look. can you do that?" and "i mean only the theme, not functionality, features".

Context: the owner saw the RC2 preview screens and did not like the surya look (decision 22's "floating rounded panels on a soft canvas", PR #2's light-first themes, PR #36). This supersedes the layout paragraph of decision 22 and the theme parts of PR #2.

- Default theme is comet's own dark ("Zeron Dark", id comet_dark) with comet's light as the light option; system-follow stays.
- The surya chrome (canvas inset, floating panels, composer pill restyle, title typography, sidebar cards) is removed from the shell; the new panes (tasks, files, browser, inbox) take comet's tokens.
- Every feature stays. The motion switch stays (it is a function, not a look).
- The surya themes and `surya.rs` stay in the tree, selectable but not default, so a reversal is cheap.
- Owner: surya-theme, branch fix/comet-look, before the 2026-09-06 11:45 freeze; proof = side-by-side with upstream comet at the import commit.
- Amended 19:32, owner verbatim: "the new gui should look like surya's comet, but with our feature built in". So comet's own elements keep comet's exact shape, including its rounded glass question panel that replaces the composer (states had found it is comet's, not ours); permissions mirror that panel. What goes is only what surya added on top: the needs-you cards over the transcript, the surya chrome and tokens. The 19:26 "box/modal, remove them" refers to those additions.

## 27. Zero-copy stays off; the frame-rate work targets the clock and the present cadence

Coordinator ruling (raven), 2026-09-05 21:13, under the owner's 06:17 autonomy order; the owner read the advice at 20:50 and said "fold your recommendation on the zero-copy".

Measured on the owner's RTX 4080 (PR #85, docs/probes/windows-zero-copy-2026-09-05.md), same animating page, 45 to 50 s each:

| path | main-thread cost per frame |
|---|---|
| zero-copy on (GPU copy inside CEF's paint callback) | avg 1.39 ms, max 10.38 ms |
| zero-copy off (CPU copy + upload, the shipping path) | avg 0.13 ms, max 0.41 ms |

The cost of the new path is the wait for the GPU to finish the copy. CEF's contract forces that wait (cef_render_handler.h 161-167: the pooled texture cannot be touched after the callback returns). The seat's probe showed its own texture-to-texture copy on the same context waits as long, so the stall is GPU scheduling across three contexts, not something the app can skip. At the RC pane size (518x786, about 1.6 MB a frame) the CPU copy is trivial. Neither path is where frames go missing at 120 Hz (8.3 ms budget).

- PR #85 merges with `SURYA_BROWSER_ZERO_COPY` default OFF. The mechanism stays in the tree as the proven fallback with its counters and probe.
- No more seat time on zero-copy before the RC. The remaining post-RC items on it: `passed()` treats any error HRESULT as device-lost (d3d11.rs:194); the fork's four minor notes from gpui-surya#1.
- The frame-rate work (surya-browser-perf, then the Codex gpt-6-astra xhigh seat) targets the levers haktui's spike measured (haktui docs/spike-scroll-frame-rate-2026-08-28.md): own high-resolution frame clock (Windows thread-pool timers tick at 15.6 ms, so a 16 ms timer fires every 31 ms), CEF windowless frame rate = display rate, DXGI maximum frame latency 1 via a second fork PR, then presenting on the vsync beat. Baseline first: shown frames out of 120 on the owner's machine, measured with #86's instruments on main with #85 in, before any lever moves. The xhigh seat's brief names the cadence gap as its target, not the copy.
- AMENDED 21:47, owner verbatim: "i thought we're chasing the slow zero-copy gpu frames, scroll frame-rate work and agent's cdp now? i want them in the RC". So all three are RC scope, not post-RC: the zero-copy cost is chased now (the lever to test first: run CEF's UI thread off gpui's main thread, `multi_threaded_message_loop`, so the GPU wait no longer blocks the app's frame; then pane-size gating), the frame-rate levers land as PRs tonight (perf's PR 2, the fork latency patch, present on the vsync beat), and agent CDP (#87) merges tonight. The Codex gpt-6-astra xhigh seat is spawned NOW, in parallel with surya-browser-perf, cef2 coordinating the shared files. What is green by the 11:45 freeze ships; a flag defaults ON only where the owner's machine measures it better, otherwise it ships in the tree behind the flag. The baseline-first rule stands for the frame-rate levers.
- Zero-copy earns its keep only at large pane sizes: the CPU path grows with pixel count (a 4K pane is about 33 MB a frame, several ms), the GPU wait does not. If the browser pane ever fills a 4K display, turn the flag on by pane size. Post-RC item.

## 28. Windows is the product, Mac next, Linux is a test bench only

Owner ruling, 2026-09-05 22:23, verbatim: "btw i mainly gonna use surya on windows, and in the future mac, but never on linux. linux can stay as testing ground, but its not proof for RC".

- An RC gate is proven on the owner's Windows machine (dtry) or it is not proven. A :7 Linux run is smoke, useful to catch a crash early, never the evidence a PR ships on.
- Every RC-scoped feature gets a dtry proof before the freeze: comet look (#82) shots at both sizes, one browser per tab (#83), agent CDP (#87), zero-copy on/off pair (#85), frame-rate baseline and levers (#86, perf PR 2, astra), permission chip (states). One seat on the dtry GUI slot at a time, announced over agb, cef2 arbitrates.
- Linux packaging and sandbox (#80) are not RC gates. They merge when green as test-bench infrastructure, and no further seat time goes to Linux-only polish before the RC.
- Mac is the next platform after the RC; nothing tonight targets it.
- Amends decision 25's proof clause and every "proof on :7" line in briefs written today.

## 29. One end-to-end acceptance test on Windows before the RC, while the owner sleeps

Owner order, 2026-09-05 22:43, verbatim: "once all done, do one end-to-end test, testing every button, surface, features etc. to make sure everything works exactly as planned. do the test when im asleep".

- Runs on dtry (decision 28) on the newest main build after the browser-wave merges, engine reinstalled to the same sha, once the owner is off dtry (he says so, or no owner input on dtry after 01:00).
- One Codex gpt-6-astra (high) tester seat (owner 22:45: "model for e2e tester is astra, make sure he has the right tool for it windows-dtry mcp, maybe /zoom") drives the app over the windows-dtry MCP, zoom CLI for fine detail, through every rail entry, page, button and feature (list in /tmp/surya-e2e-acceptance.md), a screenshot per step into the gallery, a PASS/FAIL row per step in docs/acceptance/e2e-2026-09-06.md, FAILs filed to raven as they appear; the owning seats fix, raven re-runs the failed steps.
- Supersedes the 22:07 note that deferred the acceptance round to 04:00-08:00: the trigger is "merges done and owner asleep", not the clock.

## 30. The 13:00 deadline is scrapped; the RC ships when it is complete

Owner ruling, 2026-09-06 00:52, verbatim: "scrape the deadline. take as many time as you want to build RC. list all the things not included in RC".

- No clock gate any more: the 11:45 freeze, the 12:15 FINAL MAIN and the 13:00 ship are gone (the freeze cron was deleted at 00:53).
- The RC ships when every RC-scoped PR is merged with a dtry proof (decision 28), the end-to-end acceptance run (decision 29) is green, and the rename (decision 23) has landed. The sequence is unchanged (freeze -> rename PR -> FINAL MAIN -> final round -> RC note -> Taildrop -> retire seats); only its trigger changed from the clock to "done".
- "Ship un-renamed if the rename is late" is withdrawn: the rename lands before the RC.
- The not-in-RC list below is the owner's to pull from; anything he names moves into RC scope.

## 31. The windows-dtry MCP is open for any seat, no per-call approval

Owner, 03:23 on 2026-09-06, verbatim: "use windows mcp however you want".

- The earlier standing rule (stop and ask the owner before every windows-dtry call; prefer local scripts) is withdrawn.
- Any seat may drive dtry through the windows-dtry MCP for proofs, shots and input, subject only to the GUI arbiter (decision 28: one GUI session at a time, surya-cef3 grants the slot).
- The schtasks /it + conhost --headless route for shots still works and stays valid; seats pick whichever is faster. Input via the MCP injection remains the reliable path (SendKeys never reaches the gpui window).

## 32. The mockup is deleted

Owner, 04:01 on 2026-09-06: "the mockup was a typescript backend with react frontend web app, nothing to do with current version of surya. in fact, you can delete them now".

- `mockup/` (134 files) is removed from main. It was the pre-fork web prototype, not a design reference for the GPUI app (decision 26 already made comet's look the reference).
- Any future design sandbox is a new decision, not a revival of this tree.

## 33. CI leaves pc-ajim: Linux on GitHub-hosted runners, Windows on dtry

Owner ruling, 2026-09-07 02:05, "follow your recommendation", on his two conditions, verbatim: "the low priority run dont effect my non-gaming day to day use" and "it wont eat too much space".

Supersedes decision 24, which put CI on two self-hosted Linux runners on pc-ajim because GitHub-hosted jobs were blocked on billing. Decision 24 stays on the record for why that happened and is no longer the arrangement.

- The Linux checks go back to `ubuntu-latest`, free once the repository is public. The job carries `if: github.event.repository.visibility == 'public'`, so the workflow can merge before the repository is flipped without ever asking for a billed minute. An absent field is not "public" either, so the failure direction is "does not run", never "runs and bills".
- The hosted-era steps come back with it: freeing disk and `apt-get` for gpui's system libraries. Decision 24 banned both, and that ban still holds for anything running on pc-ajim. A GitHub-hosted runner is a throwaway VM, not the owner's machine.
- The Windows build moves to dtry, label `surya-win`, installed by `deploy/windows/install-runner.ps1`. It runs `deploy/windows/build.ps1` at Idle priority on 8 of 16 threads, reuses `E:\surya-remote-target`, deletes each run's `dist` folder, keeps the zip as a 14-day artifact, and runs `cargo clean` only when the target passes 25 GB and only once a week. Those five rules are the owner's two conditions written down.
- The `windows` job never runs for a pull request from a fork. A self-hosted runner executes the PR author's code on the owner's PC; once the repository is public, anyone could open one.
- Only `actions/*` actions may be used. The repository's Actions policy rejects anything else before a job starts, which is why the Linux cache is a hand-written `actions/cache` and not `Swatinem/rust-cache`.
- pc-ajim's two runners retire by hand after this workflow has one green run, not before. `browser-nightly.yml` still needs `[self-hosted, surya-ci]` and the CEF distribution at `/store/surya-ci-cef`, so where the nightly runs is a separate decision.
- Windows is still the product and Linux still the test bench (decision 28). Nothing here makes a green Linux run RC proof.

## Open

None at day zero.
