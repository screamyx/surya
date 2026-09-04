# Probe: driving Claude Code headless, the four controls surya needs

Date: 2026-09-05, 01:57 to 02:02.
Claude Code 2.1.260, raw CLI, `--input-format stream-json --output-format stream-json --verbose`, model claude-haiku-4-5-20251001.
Driver: `headless-controls-driver.py` beside this file. Every run had the agb env vars unset so no probe stole the session's agb identity.

## Answers

| Question | Answer | Evidence |
|---|---|---|
| Does `/model sonnet` mid-session switch the next turn? | Yes | Turn 1 replied "Set model to `Sonnet 5` for this session only" with 0 turns. Turn 2's assistant message carried `model: claude-sonnet-5`. asked=1 switched=1 |
| Does AskUserQuestion reach the host headless? | Yes, only with `--permission-prompt-tool stdio` | Without the flag the tool is absent from the init tool list (121 of 121 tools, no AskUserQuestion, no EnterPlanMode) and the model says "I don't have access to an AskUserQuestion tool". With the flag: 124 tools, both present. The question arrives as `control_request` subtype `can_use_tool`, `tool_name: AskUserQuestion`, `requires_user_interaction` in the keys. Answering on stdin with `control_response` `behavior: allow` and `updatedInput.answers = { "<question>": "<label>" }` produced the tool result "Your questions have been answered" and the reply "You chose A". asked=1 control_requests=1 |
| Does `--resume <id>` continue after the process died? | Yes | Session 1 stored "pineapple" and exited. A new process with `--resume` answered "pineapple". asked=1 remembered=1 |
| Does a control interrupt stop a running command? | Yes, in 2 seconds | A foreground `until` loop was running (loop_before=2 processes). `control_request` subtype `interrupt` returned success, the tool result became "The user doesn't want to proceed with this tool use", the result event came 2.0 s after the interrupt, loop_after=1 (only the pgrep itself). The first attempt used `sleep 40`, which this box's hook blocks, so it proved nothing; the second attempt is the one that counts |

## Flags the daemon must pass

```
claude -p \
  --input-format stream-json --output-format stream-json --verbose \
  --include-partial-messages \
  --permission-prompts host --permission-prompt-tool stdio \
  --resume <session-id when reattaching>
```

`--permission-prompt-tool stdio` is what the Agent SDK passes when a `canUseTool` callback is set (read in `@anthropic-ai/claude-agent-sdk@0.3.260` sdk.mjs: `Z.push("--permission-prompt-tool","stdio")`).
Without it, permission asks and AskUserQuestion never reach the host.

## Also learned

- The SDK sends an `initialize` control request first. Its response carries the full slash command list with descriptions, so the palette can be built from it, and `pending_permission_requests` plus `pending_user_dialog_requests` so a client that reconnects can re-arm anything the loop is blocked on.
- The initialize request accepts `supportedDialogKinds` and `perTaskStopAffordance`. Without `perTaskStopAffordance` an interrupt also kills background tasks the agent started.
- Headless `/model` prints "Usage: /model <name>. Available: sonnet, opus, haiku, fable, best, sonnet[1m], opus[1m], fable[1m], opusplan, default, or a full model ID". Headless `/mcp` prints a one-line count and "Use /mcp in the terminal for details". `/plugin` is not in the headless command list at all.
- A one-shot `-p "text"` run reports 122 tools; an open-input run without the stdio prompt tool reports 121. Neither has AskUserQuestion.
