# Product

Conductor

## What it is

Conductor is a desktop and cloud environment for running multiple coding agents in isolated, Git-backed workspaces ([Conductor Docs](https://www.conductor.build/docs); [Conductor home capture](shots/conductor-home.png)).
Each workspace has its own branch, files, terminal, diff, and review flow, and it can run Claude Code, Codex, Cursor, or OpenCode agents ([Conductor Docs](https://www.conductor.build/docs); [Conductor home](https://www.conductor.build/)).

## Job 1: Start work from one sentence

The documented entry choices are "Open project", "Open GitHub project", and "Quick start", after which a workspace can begin from a branch, pull request, GitHub issue, or Linear issue ([Your first workspace](https://www.conductor.build/docs/first-workspace); [first workspace capture](shots/conductor-first-workspace.png)).
The first chat supports attached files, folders, comments, notes, specifications, screenshots, and logs ([Your first workspace](https://www.conductor.build/docs/first-workspace)).
Flow ([Your first workspace](https://www.conductor.build/docs/first-workspace)):

1. Choose a local project, GitHub project, or quick start.
2. Select the starting branch or linked work item.
3. Open a chat with a supported agent.
4. Enter the request and attach relevant context.

## Job 2: Watch an agent work

The product site's hero image is labeled as the Conductor app with collaborative agent chats and a changes panel ([Conductor home](https://www.conductor.build/); [Conductor home capture](shots/conductor-home.png)).
The workspace documentation identifies chat, terminal, diff, and checks as the main work surfaces but does not document whether hidden thinking is shown or repeated tool noise is collapsed ([Your first workspace](https://www.conductor.build/docs/first-workspace)).
The live desktop application was not observed because the available research host did not provide the supported macOS desktop environment.
Flow ([Your first workspace](https://www.conductor.build/docs/first-workspace); [Conductor Docs](https://www.conductor.build/docs)):

1. Open the workspace chat.
2. Follow agent messages.
3. Use the terminal and changed-files views for execution evidence.
4. Move to review when the task is ready.

## Job 3: Answer a question or permission

Local agents run with the user's operating-system permissions, and tool calls may ask for approval before the agent continues for shell, file, MCP, web, or other operations ([Security and permissions](https://www.conductor.build/docs/reference/security-and-permissions); [permissions capture](shots/conductor-security-permissions.png)).
Cloud agents instead run inside isolated Linux sandboxes ([Security and permissions](https://www.conductor.build/docs/reference/security-and-permissions)).
The exact approval controls, prompt labels, and waiting-state presentation were not observed because the live desktop application was not available.
Flow ([Security and permissions](https://www.conductor.build/docs/reference/security-and-permissions)):

1. The local agent proposes a guarded tool call.
2. Conductor may ask for approval before continuation.
3. The user answers the request.
4. The agent continues under the local user's permissions.

## Job 4: Review the result

The "Diff Viewer" lists changed files, supports unified diffs and commit filtering, and places "Create PR" in the review surface ([Diff Viewer](https://www.conductor.build/docs/reference/diff-viewer); [Diff Viewer capture](shots/conductor-diff-viewer.png)).
Line comments send precise feedback to an agent, and GitHub review comments can appear in the same review context ([Diff Viewer](https://www.conductor.build/docs/reference/diff-viewer)).
Saved "Run" commands can start tests, watchers, applications, or web servers, while the "Checks" tab collects Git, continuous-integration, deployment, comment, and todo status ([Testing](https://www.conductor.build/docs/concepts/testing); [testing capture](shots/conductor-testing.png); [Your first workspace](https://www.conductor.build/docs/first-workspace)).
Flow ([Diff Viewer](https://www.conductor.build/docs/reference/diff-viewer); [Your first workspace](https://www.conductor.build/docs/first-workspace)):

1. Open "Diff Viewer".
2. Filter and inspect changed files.
3. Leave line comments or send suggested fixes to the agent.
4. Run tests and checks.
5. Choose "Create PR", merge, and archive.

## Job 5: Run many agents at once

Conductor distinguishes independent work in separate workspaces from shared work by multiple agents in one workspace ([Parallel agents](https://www.conductor.build/docs/concepts/parallel-agents); [parallel agents capture](shots/conductor-parallel-agents.png)).
Separate workspaces isolate branch, files, running environment, and review, while agents in one workspace share branch, code state, and context ([Parallel agents](https://www.conductor.build/docs/concepts/parallel-agents)).
The documentation warns that agents in one workspace can edit the same files concurrently ([Parallel agents](https://www.conductor.build/docs/concepts/parallel-agents)).
A specific "needs you" label or queue was not observed in the first-party pages reviewed.
Flow ([Parallel agents](https://www.conductor.build/docs/concepts/parallel-agents)):

1. Decide whether tasks share code state.
2. Create separate workspaces for independent pull requests or multiple chats for shared work.
3. Run the agents concurrently.
4. Review and merge each independent stream on its own schedule.

## Job 6: Files and preview

Every workspace has isolated files, a terminal, a diff, and a review context, while saved "Run" commands can launch a web server or application from the workspace toolbar ([Conductor Docs](https://www.conductor.build/docs); [Testing](https://www.conductor.build/docs/concepts/testing); [testing capture](shots/conductor-testing.png)).
Conductor assigns `$CONDUCTOR_PORT` so multiple workspace applications can run at once ([Testing](https://www.conductor.build/docs/concepts/testing)).
A dedicated embedded app-preview surface and its relationship to chat were not observed in the reviewed documentation.
Flow ([Your first workspace](https://www.conductor.build/docs/first-workspace); [Testing](https://www.conductor.build/docs/concepts/testing)):

1. Work in the workspace chat and files.
2. Use the terminal or a saved "Run" command.
3. Start the application on its workspace port.
4. Review file changes in "Diff Viewer".

## Job 7: Work from a phone

The product home page says work can be conducted from desktop app, mobile, or API, but its iOS control is currently labeled "SOON" ([Conductor home](https://www.conductor.build/); [Conductor home capture](shots/conductor-home.png)).
A usable phone workflow and any reduced mobile feature set were not observed in a live product or first-party guide.
Flow: not observed because the current site advertises mobile access while simultaneously labeling iOS as forthcoming ([Conductor home](https://www.conductor.build/)).

## Job 8: Persistence and resume

Local chats, workspaces, and repository files are stored on the local machine, while cloud session and repository data are stored and synchronized in Conductor's managed infrastructure ([Security and permissions](https://www.conductor.build/docs/reference/security-and-permissions); [permissions capture](shots/conductor-security-permissions.png)).
The documented workspace lifecycle ends with review, pull request, merge, and archive ([Conductor Docs](https://www.conductor.build/docs); [Your first workspace](https://www.conductor.build/docs/first-workspace)).
The exact behavior during a disconnect and the resume controls were not observed.
Flow ([Conductor Docs](https://www.conductor.build/docs); [Security and permissions](https://www.conductor.build/docs/reference/security-and-permissions)):

1. Reopen Conductor.
2. Select the local or cloud workspace.
3. Continue its chat and Git-backed work.
4. Archive the workspace after merge.

## Three things it does better than our mockup

1. Conductor explicitly teaches when agents should share one workspace and when independent work requires separate workspaces, while Surya's agent hierarchy shows parent and child agents without explaining the isolation decision ([parallel agents capture](shots/conductor-parallel-agents.png); [Surya agents](../../mockup/shots/agents-desktop.png)).
2. Conductor's review surface combines the changed-file list, commit filtering, line comments, agent feedback, and pull-request creation, while Surya separates files, preview feedback, and final result across routes ([Diff Viewer capture](shots/conductor-diff-viewer.png); [Surya files](../../mockup/shots/files-desktop.png); [Surya preview](../../mockup/shots/preview-desktop.png); [Surya result](../../mockup/shots/result-desktop.png)).
3. Conductor gives every workspace its own branch, files, terminal, running environment, and review lifecycle, while Surya's workspace summary emphasizes agent counts and statuses without exposing the same workspace-isolation model ([first workspace capture](shots/conductor-first-workspace.png); [parallel agents capture](shots/conductor-parallel-agents.png); [Surya agents](../../mockup/shots/agents-desktop.png)).
