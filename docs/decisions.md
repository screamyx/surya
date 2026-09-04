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

## Open

- Does the AG-UI runtime abort a run when the browser disconnects?
  Being probed on 2026-09-05.
  Decides whether runs live inside HTTP requests or beside them.
