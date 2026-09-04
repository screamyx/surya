# Product

## Windsurf

## What it is

The former Windsurf site now redirects to Devin Desktop, while the product-owned docs still document "Cascade" as a legacy local agent within the renamed desktop product. [Devin Desktop](https://devin.ai/desktop) [Cascade Overview](https://docs.devin.ai/desktop/cascade/cascade)
The current desktop product combines Cascade, Devin Local, cloud Devin sessions, an editor, browser previews, and an Agent Command Center. [Agent Command Center](https://docs.devin.ai/desktop/agent-command-center) [windsurf-desktop-home.png](shots/windsurf-desktop-home.png)

## Job 1 - Start work from one sentence

Cascade opens with "Cmd/Ctrl+L," automatically includes selected editor or terminal text, and puts model and mode selectors below the conversation input. [Cascade Overview](https://docs.devin.ai/desktop/cascade/cascade) [windsurf-cascade-overview.png](shots/windsurf-cascade-overview.png)
"Code mode" is documented as the default for implementation, while "Plan mode" explores the codebase, asks clarifying questions, offers choices, and produces a persistent Markdown plan with an "Implement" action. [Cascade Modes](https://docs.devin.ai/desktop/cascade/modes) [windsurf-modes-plan.png](shots/windsurf-modes-plan.png)

1. Open Cascade and enter a sentence, optionally with selected code or terminal text already attached. [Cascade Overview](https://docs.devin.ai/desktop/cascade/cascade)
2. Keep the default "Code mode" for implementation or choose "Plan mode" for a scoped plan. [Cascade Modes](https://docs.devin.ai/desktop/cascade/modes)
3. In Plan mode, answer the clarifying choices and select "Implement" when the plan is ready. [Cascade Modes](https://docs.devin.ai/desktop/cascade/modes)

## Job 2 - Watch an agent work

Cascade creates a Todo list inside the conversation for complex tasks, while a separate planning agent refines the longer plan in the background. [Cascade Overview](https://docs.devin.ai/desktop/cascade/cascade)
Tool calls include Search, Analyze, Web Search, MCP, and terminal work, and a stopped trajectory exposes a "continue" action plus an optional Auto-Continue setting. [Cascade Overview](https://docs.devin.ai/desktop/cascade/cascade)
The public docs did not show whether model reasoning is exposed or exactly how noisy tool output is collapsed. [Cascade Overview](https://docs.devin.ai/desktop/cascade/cascade)

1. Cascade creates or updates its Todo list and chooses a tool. [Cascade Overview](https://docs.devin.ai/desktop/cascade/cascade)
2. The tool call appears in the conversation, where tool-specific controls such as "Accept" or "Auto-fix" can appear. [Cascade Overview](https://docs.devin.ai/desktop/cascade/cascade)
3. If the trajectory stops at its tool-call limit, the user selects "continue" or relies on Auto-Continue. [Cascade Overview](https://docs.devin.ai/desktop/cascade/cascade)

## Job 3 - Answer a question or permission

Plan mode can ask multiple-choice clarifying questions, and the finished plan asks to begin implementation or offers an "Implement" button. [Cascade Modes](https://docs.devin.ai/desktop/cascade/modes)
The current permission selector documents "Normal mode" as the default, where reads run automatically and writes and shell commands wait for explicit approval. [Permissions](https://docs.devin.ai/cli/reference/permissions)
Approval choices can persist once, for the session, for the project, locally for the project, or globally, and command prompts can also offer "Edit command" and "Describe change to command." [Permissions](https://docs.devin.ai/cli/reference/permissions)

1. The agent presents a clarification choice, plan transition, or permission prompt. [Cascade Modes](https://docs.devin.ai/desktop/cascade/modes) [Permissions](https://docs.devin.ai/cli/reference/permissions)
2. The run waits for an explicit decision when the selected permission mode requires one. [Permissions](https://docs.devin.ai/cli/reference/permissions)
3. The user approves, denies, edits, or rewrites the command and can choose how long an approval persists. [Permissions](https://docs.devin.ai/cli/reference/permissions)

## Job 4 - Review the result

"Quick Review" runs a separate review agent on local changes and returns diff feedback inside the editor, but the current documentation says it is available only to Devin Local and not legacy Cascade. [Quick Review](https://docs.devin.ai/desktop/quick-review) [windsurf-quick-review.png](shots/windsurf-quick-review.png)
Cascade itself can deploy a web project through "App Deploys," returning a public URL and claim link after the tool analyzes, uploads, builds, and deploys the project. [App Deploys](https://docs.devin.ai/desktop/cascade/app-deploys)

1. For current local-agent work, select "Quick Review" and choose a review model. [Quick Review](https://docs.devin.ai/desktop/quick-review)
2. Inspect the independent agent's diff feedback directly in the editor. [Quick Review](https://docs.devin.ai/desktop/quick-review)
3. For a Cascade web project, ask to deploy, inspect the public result, and claim the deployment when it should persist under the user's account. [App Deploys](https://docs.devin.ai/desktop/cascade/app-deploys)

## Job 5 - Run many agents at once

The current Agent Command Center uses a Kanban board with the exact columns "Running," "Waiting for review," and "Done." [Devin Desktop](https://devin.ai/desktop) [windsurf-desktop-home.png](shots/windsurf-desktop-home.png)
Individual cards and the sidebar expose secondary states including "Working...", "PR is ready," and "Waiting for CI," and sessions can be filtered, sorted, and grouped by workspace or Space. [Devin Desktop](https://devin.ai/desktop) [Agent Command Center](https://docs.devin.ai/desktop/agent-command-center)
The docs say native OS notifications fire when a session finishes or "needs your input," while a running session is greyed out and read-only. [Agent Command Center](https://docs.devin.ai/desktop/agent-command-center) [windsurf-agent-command-center.png](shots/windsurf-agent-command-center.png)

1. Open the Agent Command Center to see local and cloud sessions together. [Agent Command Center](https://docs.devin.ai/desktop/agent-command-center)
2. Scan the board from "Running" through "Waiting for review" to "Done," or filter the session sidebar. [Devin Desktop](https://devin.ai/desktop)
3. Open a ready session for review after its native notification arrives. [Agent Command Center](https://docs.devin.ai/desktop/agent-command-center)

## Job 6 - Files and preview

Devin Desktop keeps an editor beside the agent and can open a local web app as a built-in browser pane or editor tab. [Devin Desktop Previews](https://docs.devin.ai/desktop/previews) [windsurf-browser-previews.png](shots/windsurf-browser-previews.png)
The preview's "Send element" control adds selected UI elements or console errors to the pending agent prompt as an @ mention. [Devin Desktop Previews](https://docs.devin.ai/desktop/previews)

1. Ask the agent to preview the local site, which opens the preview through a tool call. [Devin Desktop Previews](https://docs.devin.ai/desktop/previews)
2. Keep the preview beside the agent and editor while inspecting the running app. [Devin Desktop Previews](https://docs.devin.ai/desktop/previews)
3. Select "Send element" and click a component so its DOM context enters the agent prompt. [Devin Desktop Previews](https://docs.devin.ai/desktop/previews)

## Job 7 - Work from a phone

Not observed because the public Windsurf and Devin Desktop documentation did not show a mobile agent session or document a native mobile controller. [Devin Desktop](https://devin.ai/desktop)
The mobile-width public page is a marketing layout rather than the authenticated agent workspace, so it does not establish what an actual phone workflow keeps or cuts. [Devin Desktop](https://devin.ai/desktop)

1. A mobile agent entry point was not observed in the product-owned public material. [Devin Desktop](https://devin.ai/desktop)
2. Session supervision and review behavior on a phone therefore remain not observed. [Agent Command Center](https://docs.devin.ai/desktop/agent-command-center)
3. No claim about a mobile resume flow is made from the desktop-only evidence. [Agent Command Center](https://docs.devin.ai/desktop/agent-command-center)

## Job 8 - Persistence and resume

Cascade plan files persist outside the repository and can be @ mentioned into a fresh context to continue implementation across sessions. [Cascade Modes](https://docs.devin.ai/desktop/cascade/modes)
The current Agent Command Center sidebar lists every session in a workspace and supports grouping and renaming, while Spaces group sessions, pull requests, files, and context around a project. [Agent Command Center](https://docs.devin.ai/desktop/agent-command-center)
Disconnect behavior for a running local Cascade was not observed in the public documentation. [Cascade Overview](https://docs.devin.ai/desktop/cascade/cascade)

1. Save or finish a Plan mode plan, which remains available outside the repository. [Cascade Modes](https://docs.devin.ai/desktop/cascade/modes)
2. Start a fresh conversation and @ mention the saved plan file. [Cascade Modes](https://docs.devin.ai/desktop/cascade/modes)
3. Select "Implement" to continue from the plan, while the Agent Command Center keeps the related sessions grouped in a Space. [Cascade Modes](https://docs.devin.ai/desktop/cascade/modes) [Agent Command Center](https://docs.devin.ai/desktop/agent-command-center)

## Three things it does better than our mockup

1. Its board makes operational state the composition, with "Running," "Waiting for review," and "Done" as primary columns instead of presenting equal-weight summary tiles above sparse content. [Devin Desktop](https://devin.ai/desktop) [windsurf-desktop-home.png](shots/windsurf-desktop-home.png)
2. Its preview is not a passive screenshot because "Send element" turns a visual selection or console error into structured prompt context beside the agent. [Devin Desktop Previews](https://docs.devin.ai/desktop/previews) [windsurf-browser-previews.png](shots/windsurf-browser-previews.png)
3. Its Plan mode creates a durable implementation artifact with explicit clarification choices and an "Implement" transition, giving long work a stronger resume path than a transient chat. [Cascade Modes](https://docs.devin.ai/desktop/cascade/modes) [windsurf-modes-plan.png](shots/windsurf-modes-plan.png)
