# Product

Kiro

## What it is

Kiro is an IDE, CLI, web, and early-access mobile agent environment built around chat, structured specs, hooks, and cloud sessions.
Its specs turn an idea into requirements, design, and task files before or alongside implementation.
Source: "Docs", https://kiro.dev/docs/, and "Specs", https://kiro.dev/docs/specs/.

## Job 1 - Start work from one sentence

Flow: 1. Open chat. 2. Choose the optional "Spec", "Plan", "Bug Fix", or "Quick Spec" workflow. 3. Describe the feature in natural language. 4. Review requirements, design, and tasks. 5. Run one task or all tasks.
The resting chat welcome is "Let's build", and skipping the workflow selector uses the "Default" agent.
For a feature spec, Kiro asks whether the user is developing a feature or fixing a bug, then offers "Requirements-First" or "Design-First" for a feature.
The generated artifacts are `requirements.md` or `bugfix.md`, `design.md`, and `tasks.md`.
Source: "Chat", https://kiro.dev/docs/ide/chat/, "Specs", https://kiro.dev/docs/specs/, and "Your first project", https://kiro.dev/docs/getting-started/first-project/, screenshot `docs/research/shots/kiro-first-spec.png`.

## Job 2 - Watch an agent work

Flow: 1. Keep chat beside the editor. 2. Watch task state and the execution history. 3. Open the task list for ongoing and completed work. 4. Search or restore a session when more detail is needed.
Spec tasks update to the exact states "In Progress" and "Done".
The IDE keeps an execution history of code changes, commands, search results, file operations, and other actions.
Raw reasoning and a specific noise-collapsing treatment were not observed in the official material.
Source: "Your first project", https://kiro.dev/docs/getting-started/first-project/, screenshot `docs/research/shots/kiro-first-spec.png`, and "Chat", https://kiro.dev/docs/ide/chat/.

## Job 3 - Answer a question or permission

Flow: 1. A tool requires approval. 2. A prompt appears in chat. 3. Choose "Allow", "Always allow", "Deny", or "Always deny" when persistent choices are available. 4. For a persistent rule, choose its pattern and whether it applies to all workspaces, this workspace, or this session. 5. The tool proceeds or remains blocked.
The one-time choices act only on the current invocation, while persistent choices write a user or workspace rule or retain it in session memory.
On Kiro Web, interactive per-action approvals do not apply because the agent runs in an isolated cloud sandbox.
Source: "Permissions", https://kiro.dev/docs/permissions/, screenshot `docs/research/shots/kiro-approval-flow.png`.

## Job 4 - Review the result

Flow: 1. Review generated tasks or changes in the IDE. 2. Use source control or `#Git Diff` to inspect current changes. 3. Ask Kiro Web to open a pull request. 4. Continue feedback on the pull request. 5. Kiro pushes updates.
Kiro Web creates a feature branch, commits the changes, opens a pull request with a description, and responds to later pull request comments.
The IDE source-control view can stage changes and generate an editable commit message before committing.
A dedicated Kiro diff-review screen and a live application run were not observed in the public material.
Source: "Working with the agent", https://kiro.dev/docs/web/using-the-agent/, and "Source Control", https://kiro.dev/docs/ide/editor/source-control/.

## Job 5 - Run many agents at once

Flow: 1. Start independent cloud sessions. 2. Let them run concurrently. 3. Organize related sessions into named groups or leave them under "Ungrouped". 4. Filter by group. 5. Open a session to steer or review it.
Kiro documents up to 10 concurrent cloud sessions and supports creating, resuming, and steering them from IDE, CLI, Web, and Mobile.
The web sessions panel supports "Create group", "Move to group", "Remove from group", and bulk moves on the Sessions page.
The public material did not expose a distinct "needs you" label or a full set of run-status words for the grouped list.
Source: "Cloud sessions", https://kiro.dev/docs/cloud-sessions/, screenshot `docs/research/shots/kiro-cloud-sessions.png`, and "Working with the agent", https://kiro.dev/docs/web/using-the-agent/, screenshot `docs/research/shots/kiro-session-groups.png`.

## Job 6 - Files and preview

Flow: 1. Navigate files in "Explorer". 2. Edit in the central editor. 3. Keep the "Chat Panel" beside it. 4. Use "Source Control" to view changes and commit. 5. Use split views when comparing files.
Kiro's documented IDE is an editor-first surface with project files, search, source control, terminal, and chat in one shell.
A built-in live application preview was not observed in the official interface material.
Source: "Kiro Interface", https://kiro.dev/docs/ide/editor/interface/, screenshot `docs/research/shots/kiro-interface.png`.

## Job 7 - Work from a phone

Flow: 1. Request early access through TestFlight. 2. Sign in. 3. Open the synchronized session list. 4. Start or steer a cloud session from the phone. 5. Reattach from another surface later.
Kiro for iOS is in early access, and its page says sessions and preferences sync between Kiro Web and iOS.
Detailed mobile documentation is explicitly "coming soon", so the controls removed from the phone experience were not observed.
Source: "Mobile", https://kiro.dev/docs/mobile/, screenshot `docs/research/shots/kiro-mobile.png`.

## Job 8 - Persistence and resume

Flow: 1. Start a cloud session. 2. Close the client or laptop. 3. Let the task continue in the sandbox. 4. Reattach from Web, Mobile, IDE, or CLI. 5. Replay the transcript and resume from the retained repository and file state.
Conversation history, bound repositories, and sandbox file state persist in the cloud, and a waiting approval is shown to the next attached client.
The September 2026 changelog also documents a CLI session dashboard for browsing, searching, resuming, and cleaning up local and cloud sessions.
Source: "Cloud sessions", https://kiro.dev/docs/cloud-sessions/, screenshot `docs/research/shots/kiro-cloud-sessions.png`, and "Changelog", https://kiro.dev/changelog/.

## Three things it does better than our mockup

1. Kiro turns a short feature description into durable requirements, design, and tasks artifacts with explicit task states, while the mockup's new ask offers modes without a visible artifact pipeline.
Evidence: `docs/research/shots/kiro-first-spec.png` and `mockup/src/screens/new-ask.tsx`.
2. Kiro lets one permission decision become a precise rule by pattern and scope, while the mockup's permission event presents the immediate run decision without a persistent rule builder.
Evidence: `docs/research/shots/kiro-approval-flow.png` and `mockup/src/screens/agent-feed/events.tsx`.
3. Kiro makes the same cloud session attachable from IDE, CLI, Web, and Mobile while the sandbox continues without a client, while the mockup models session reattachment inside a single agent sheet.
Evidence: `docs/research/shots/kiro-cloud-sessions.png` and `mockup/src/screens/agent-feed/sessions-sheet.tsx`.
