# Product

Cline

## What it is

Cline is an open-source coding agent available in editor, terminal, Kanban, VS Code, and JetBrains surfaces ([Cline Overview](https://docs.cline.bot/cline-overview); [overview capture](shots/cline-overview.png)).
Its core interaction is a natural-language conversation that can read and write files, run commands, and use a browser after the applicable approval ([Cline Overview](https://docs.cline.bot/cline-overview)).

## Job 1: Start work from one sentence

"Plan" mode lets the user describe a goal, lets Cline inspect the codebase, and prevents edits or command execution until the user switches to "Act" mode ([Plan & Act Mode](https://docs.cline.bot/core-workflows/plan-and-act); [Plan and Act capture](shots/cline-plan-act.png)).
The full planning context carries into "Act", and the settings can assign a different model to each mode ([Plan & Act Mode](https://docs.cline.bot/core-workflows/plan-and-act)).
Flow ([Plan & Act Mode](https://docs.cline.bot/core-workflows/plan-and-act)):

1. Start in "Plan".
2. Describe the goal in one message.
3. Let Cline inspect files and discuss the approach.
4. Switch to "Act" to modify files and run commands.

## Job 2: Watch an agent work

The overview says Cline shows file reads, edits, terminal commands, and browser actions in the conversation and requires explicit approval for actions by default ([Cline Overview](https://docs.cline.bot/cline-overview); [overview capture](shots/cline-overview.png)).
In Kanban, a running card displays the agent's latest message or tool call, and opening the card exposes the agent terminal user interface with its conversation and actions ([Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow); [Kanban capture](shots/cline-kanban.png)).
Whether hidden thinking is shown or repetitive tool noise is collapsed was not observed because the extension was not run in a compatible editor during this research.
Flow ([Cline Overview](https://docs.cline.bot/cline-overview); [Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow)):

1. Start the task.
2. Follow messages and proposed tool calls in the conversation.
3. Approve the requested action when required.
4. Open the Kanban card for the complete action history.

## Job 3: Answer a question or permission

"Auto Approve" is divided by capability into project or all-file reads, project or all-file edits, safe or all commands, browser use, MCP server use, and notifications ([Auto Approve & YOLO Mode](https://docs.cline.bot/features/auto-approve); [Auto Approve capture](shots/cline-auto-approve.png)).
The default recommendation is to auto-approve reads only and keep edits, commands, browser use, and MCP server use behind approval ([Auto Approve & YOLO Mode](https://docs.cline.bot/features/auto-approve)).
Cline's model marks terminal commands as safe or approval-required instead of using a fixed allowlist ([Auto Approve & YOLO Mode](https://docs.cline.bot/features/auto-approve)).
The exact approval prompt and its waiting-state label were not observed because the extension was not run in a compatible editor during this research.
Flow ([Auto Approve & YOLO Mode](https://docs.cline.bot/features/auto-approve)):

1. Cline proposes a tool call.
2. The capability's Auto Approve setting determines whether Cline asks.
3. The user approves a guarded action or the enabled rule lets it proceed.
4. Cline continues the task.

## Job 4: Review the result

Cline creates a checkpoint after every file modification or command and offers "Compare" plus "Restore Files", "Restore Task Only", and "Restore Files & Task" actions ([Checkpoints](https://docs.cline.bot/core-workflows/checkpoints); [checkpoints capture](shots/cline-checkpoints.png)).
Kanban card detail shows the full diff, checkpoint-range diffs, and inline diff comments that are sent back to the agent ([Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow)).
The Kanban shipping actions are labeled "Commit" and "Open PR" ([Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow); [Kanban capture](shots/cline-kanban.png)).
Flow ([Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow); [Checkpoints](https://docs.cline.bot/core-workflows/checkpoints)):

1. Open the task card.
2. Review the full or checkpoint-range diff.
3. Add inline feedback or restore a checkpoint.
4. Choose "Commit" or "Open PR" when ready.

## Job 5: Run many agents at once

Each Kanban card is a discrete task that runs in an isolated Git worktree, with the card showing the latest agent message or tool call ([Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow); [Kanban capture](shots/cline-kanban.png)).
Users can link dependencies, and completing or deleting one task can auto-start its dependent task with auto-commit enabled ([Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow)).
The documentation does not show a specific "needs you" queue or status term, so that state was not observed ([Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow)).
Flow ([Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow)):

1. Add or ask the sidebar agent to create cards.
2. Link dependencies if sequencing is required.
3. Start several independent cards.
4. Read the latest action on each card and open a card for detail.

## Job 6: Files and preview

Cline works inside VS Code or JetBrains and can read or edit the open project, use its integrated terminal, and control a browser ([Cline Overview](https://docs.cline.bot/cline-overview); [overview capture](shots/cline-overview.png)).
A dedicated live-app preview surface and its spatial relationship to chat were not observed in the reviewed first-party documentation.
Flow ([Cline Overview](https://docs.cline.bot/cline-overview); [Auto Approve & YOLO Mode](https://docs.cline.bot/features/auto-approve)):

1. Ask in the editor conversation.
2. Review proposed file actions.
3. Approve terminal or browser use when required.
4. Inspect the resulting files in the host editor.

## Job 7: Work from a phone

This job was observed only in the Kanban remote-access documentation, not on a physical phone ([Kanban Remote Access](https://docs.cline.bot/kanban/remote-access)).
Kanban binds to `127.0.0.1:3484` by default and can be exposed on the local network with `0.0.0.0`, while the documentation recommends Tailscale for access away from the host ([Kanban Remote Access](https://docs.cline.bot/kanban/remote-access)).
The page warns that the remote interface exposes the repository and agent terminal, but it does not document a reduced phone feature set ([Kanban Remote Access](https://docs.cline.bot/kanban/remote-access)).
Flow ([Kanban Remote Access](https://docs.cline.bot/kanban/remote-access)):

1. Run Kanban on the host.
2. Configure LAN binding or Tailscale.
3. Open the host address from the phone.
4. Use the Kanban web interface.

## Job 8: Persistence and resume

Checkpoints use a shadow Git repository that is separate from the project's history and persists across editor sessions ([Checkpoints](https://docs.cline.bot/core-workflows/checkpoints); [checkpoints capture](shots/cline-checkpoints.png)).
Kanban preserves a task "resume ID" so the agent session can be resumed, while deleting the card cleans up its ephemeral worktree ([Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow)).
The behavior during a network disconnect was not observed.
Flow ([Checkpoints](https://docs.cline.bot/core-workflows/checkpoints); [Kanban Core Workflow](https://docs.cline.bot/kanban/core-workflow)):

1. Reopen the editor or Kanban.
2. Select the task.
3. Resume with its saved session identity.
4. Use a checkpoint if the file or conversation state must be restored.

## Three things it does better than our mockup

1. Cline separates discussion-only "Plan" from mutating "Act", while Surya's form exposes a single "Mode" choice without showing a comparable handoff between planning and execution ([Plan and Act capture](shots/cline-plan-act.png); [Surya new ask](../../mockup/shots/new-ask-desktop.png)).
2. Cline exposes approval policy by capability, including separate read, edit, command, browser, and MCP controls, while Surya's mockup surfaces a generic "Wants approval" row and "Answer" action ([Auto Approve capture](shots/cline-auto-approve.png); [Surya agents](../../mockup/shots/agents-desktop.png)).
3. Cline makes every mutation or command a restorable checkpoint and supports file-only, task-only, or combined restore, while Surya's result and file surfaces do not show checkpoint controls ([checkpoints capture](shots/cline-checkpoints.png); [Surya files](../../mockup/shots/files-desktop.png); [Surya result](../../mockup/shots/result-desktop.png)).
