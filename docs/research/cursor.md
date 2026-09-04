# Product

## Cursor

## What it is

Cursor describes itself as a coding agent that can understand a codebase, plan and build features, fix bugs, review changes, and use development tools. [Cursor Documentation](https://cursor.com/docs)
Its current product spans a desktop editor, an agent-first desktop window, cloud workers, a web surface, and a native iOS controller. [Agents Window](https://cursor.com/docs/agent/agents-window) [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)

## Job 1 - Start work from one sentence

The public desktop demo centers the first ask in a composer labeled "Plan, search, build anything..." with a visible "Plan" mode chip and model selector. [Cursor homepage](https://cursor.com/) [cursor-home.png](shots/cursor-home.png)
The iOS start flow asks the user to choose a repository and branch before sending a task, while desktop cloud work starts by choosing "Cloud" below the agent input. [Cloud Agents](https://cursor.com/docs/cloud-agent) [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile) [cursor-mobile.png](shots/cursor-mobile.png)

1. Open the agent composer and write the task in one sentence. [Cursor homepage](https://cursor.com/)
2. Choose the repository and branch, and choose a cloud or local worker when that choice applies. [Cloud Agents](https://cursor.com/docs/cloud-agent) [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)
3. Send the task, after which the agent starts a session using the selected model and environment. [Cursor Agent](https://cursor.com/docs/agent/overview)

## Job 2 - Watch an agent work

Cursor documents file search, web search, rule lookup, file reads and edits, shell commands, browser use, image generation, and "Ask questions" as agent tools. [Cursor Agent](https://cursor.com/docs/agent/overview) [cursor-agent-overview.png](shots/cursor-agent-overview.png)
The chat timeline includes automatic checkpoints before significant changes, and a running agent accepts either a queued message or a follow-up delivered at its next tool call. [Cursor Agent](https://cursor.com/docs/agent/overview)
The public material did not show enough of a live desktop transcript to verify whether reasoning is exposed or exactly how repeated tool output is collapsed. [Cursor Agent](https://cursor.com/docs/agent/overview)

1. The agent selects a tool and performs work against the codebase or environment. [Cursor Agent](https://cursor.com/docs/agent/overview)
2. Significant edits create a checkpoint in the chat timeline. [Cursor Agent](https://cursor.com/docs/agent/overview)
3. A new message either waits below the active task or steers the run at the next tool boundary. [Cursor Agent](https://cursor.com/docs/agent/overview)

## Job 3 - Answer a question or permission

The tool list includes a control named "Ask questions," but the public page did not expose the question card, answer controls, or an explicit permission prompt. [Cursor Agent](https://cursor.com/docs/agent/overview)
The exact run state while a question or permission waits was not observed in the public material. [Cursor Agent](https://cursor.com/docs/agent/overview)

1. The agent invokes "Ask questions" when it needs user input. [Cursor Agent](https://cursor.com/docs/agent/overview)
2. The answer surface and waiting behavior were not observed because the public documentation names the tool without showing its interaction. [Cursor Agent](https://cursor.com/docs/agent/overview)
3. After the user replies, a follow-up can steer the active run at the next tool call. [Cursor Agent](https://cursor.com/docs/agent/overview)

## Job 4 - Review the result

The Agents Window has a dedicated diffs view for reviewing and committing changes and managing pull requests without leaving Cursor. [Agents Window](https://cursor.com/docs/agent/agents-window) [cursor-agents-window.png](shots/cursor-agents-window.png)
"Agent Review" can run automatically after commits, from the "/agent-review" command, or from Source Control against all local changes, with "Quick" and "Deep" depth choices. [Agent Review](https://cursor.com/docs/agent/agent-review) [cursor-agent-review.png](shots/cursor-agent-review.png)
On iOS the user can read diffs, commits, deployments, approvals, comments, and checks, then merge, publish, close, update, or change auto-merge state. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)

1. Open the diffs view or start "Agent Review" from the agent input or Source Control. [Agent Review](https://cursor.com/docs/agent/agent-review)
2. Inspect the changed files and review findings at the chosen depth. [Agent Review](https://cursor.com/docs/agent/agent-review)
3. Commit and manage the pull request in the Agents Window or complete the review and merge flow on iOS. [Agents Window](https://cursor.com/docs/agent/agents-window) [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)

## Job 5 - Run many agents at once

The public product demo groups sessions under the exact headings "IN PROGRESS" and "READY FOR REVIEW," with the current session visually leading the detail pane. [Cursor homepage](https://cursor.com/) [cursor-home.png](shots/cursor-home.png)
The Agents Window adds "Multi-workspace," "Parallel agents," cloud subagents, and isolated worktrees, and it supports moving work between local and cloud agents. [Agents Window](https://cursor.com/docs/agent/agents-window)
The docs do not name a dedicated "needs you" list, so that exact status was not observed. [Agents Window](https://cursor.com/docs/agent/agents-window)

1. Start separate agents across projects from the Agents Window. [Agents Window](https://cursor.com/docs/agent/agents-window)
2. Scan the session list by "IN PROGRESS" and "READY FOR REVIEW." [Cursor homepage](https://cursor.com/)
3. Open a session to review its plan, transcript, files, or pull request. [Cursor homepage](https://cursor.com/) [Agents Window](https://cursor.com/docs/agent/agents-window)

## Job 6 - Files and preview

The Agents Window can remain open beside the classic IDE, and users can search files without leaving it with "Ctrl+P" or search all files with "Ctrl+Shift+F." [Agents Window](https://cursor.com/docs/agent/agents-window)
The current-session demo places the conversation beside an open plan file, while the agent list remains visible in a left column. [Cursor homepage](https://cursor.com/) [cursor-home.png](shots/cursor-home.png)
Cloud agents can build and test changed software and can control a desktop and browser, but the public pages did not show a named live-preview pane equivalent to the editor. [Cloud Agents](https://cursor.com/docs/cloud-agent)

1. Select a session in the Agents Window. [Agents Window](https://cursor.com/docs/agent/agents-window)
2. Open or search a file in the adjacent editor while keeping the agent context visible. [Agents Window](https://cursor.com/docs/agent/agents-window)
3. Review changes in the diffs view, while cloud workers perform their own build, test, desktop, and browser checks. [Agents Window](https://cursor.com/docs/agent/agents-window) [Cloud Agents](https://cursor.com/docs/cloud-agent)

## Job 7 - Work from a phone

Cursor for iOS supports starting agents, watching the chat stream, sending follow-ups, reading child transcripts, reviewing pull requests, and merging from an iPhone or iPad. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile) [cursor-mobile.png](shots/cursor-mobile.png)
The mobile product deliberately omits the full editor, terminal, and file browser, showing changed files through the diff instead, and leaves environment, secret, source-control, automation, rule, skill, admin, billing, and usage configuration on the web. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)
Android uses the web agent surface as an installable PWA, while the native app documented here is for iOS and iPadOS. [Cloud Agents](https://cursor.com/docs/cloud-agent) [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)

1. Open the mobile inbox and choose an existing session or repository. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)
2. Send or steer work and follow the live transcript. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)
3. Review the diff and pull-request state, then merge or hand the session back to desktop. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)

## Job 8 - Persistence and resume

Cloud agents continue working when the local machine or phone is disconnected, and sessions started on mobile appear on web and desktop because all three surfaces share the same backend. [Cloud Agents](https://cursor.com/docs/cloud-agent) [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)
"Remote Control" moves the agent loop to the cloud while keeping terminal commands, file edits, and tests on the user's computer, and the session then appears in the mobile inbox. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)
Remote Control depends on the computer remaining awake and online, while a fully cloud-hosted agent does not have that dependency. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile) [Cloud Agents](https://cursor.com/docs/cloud-agent)

1. Start locally, on web, or on mobile, then move the run to cloud or enable "Remote Control." [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)
2. Leave the original device while the cloud agent continues, or keep the host computer available for Remote Control tool calls. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)
3. Open the same session from the mobile inbox, web agents page, or desktop Cloud Agents panel and continue directing it. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile)

## Three things it does better than our mockup

1. Cursor's real session list uses action-oriented status groups such as "IN PROGRESS" and "READY FOR REVIEW," which creates a clearer focal hierarchy than the mockup's equal stat tiles. [Cursor homepage](https://cursor.com/) [cursor-home.png](shots/cursor-home.png)
2. Cursor defines a purposeful mobile reduction rather than shrinking the IDE, keeping supervision and review while moving configuration and full file work to the web. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile) [cursor-mobile.png](shots/cursor-mobile.png)
3. Cursor treats device handoff as a first-class session capability, with one inbox across mobile, web, and desktop and a separate Remote Control path for user-owned machines. [Cursor for iOS](https://cursor.com/docs/cloud-agent/mobile) [cursor-mobile.png](shots/cursor-mobile.png)
