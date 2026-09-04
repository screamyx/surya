# Product

OpenHands

## What it is

OpenHands is an open-source agent development environment whose browser UI connects conversations, files, terminals, model settings, and automations to a persistent backend ([Agent Server Overview](https://docs.openhands.dev/openhands/usage/agent-canvas/overview)).
It can use a local process, Docker container, virtual machine, or OpenHands Cloud as that backend ([Agent Server Overview](https://docs.openhands.dev/openhands/usage/agent-canvas/overview)).

## Job 1: Start work from one sentence

The live start screen leads with "Open Repository" and "Start from Scratch", then uses "Settings" or "New Conversation" as the next action ([OpenHands live start screen](shots/openhands-live-onboarding.png)).
The repository route asks for a GitHub, GitLab, Bitbucket, or Azure DevOps connection and promises suggested tasks, while the scratch route starts without a repository ([OpenHands live start screen](shots/openhands-live-onboarding.png)).
Flow ([OpenHands live start screen](shots/openhands-live-onboarding.png); [Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)):

1. Open the app.
2. Choose a connected repository or scratch conversation.
3. Open settings for the repository route or choose "New Conversation" for scratch.
4. Enter the request in the conversation.

## Job 2: Watch an agent work

The conversation header shows a live activity chip with the current action, such as reading a file or running a command, and changes it to "Thinking" when no tool is active ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations); [conversation documentation capture](shots/openhands-docs-conversations.png)).
The activity chip disappears when the agent pauses or completes, while transcript export can include full tool details ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)).
The earlier workspace documentation shows tool-oriented work divided into "Chat Panel", "VS Code", "Terminal Tab", "App Tab", and "Browser Tab" ([Key Features](https://docs.openhands.dev/openhands/usage/key-features); [key features capture](shots/openhands-docs-features.png)).
Flow ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations); [Key Features](https://docs.openhands.dev/openhands/usage/key-features)):

1. Send the request.
2. Read the current action or "Thinking" chip.
3. Inspect chat explanations and the relevant tool surface.
4. Export the transcript when full tool detail is needed.

## Job 3: Answer a question or permission

Not observed because the live local run reached the conversation loading state while its agent-server image was still being downloaded, so no permission request or waiting behavior appeared (OpenHands local app, "Loading..." screen observed on 5 September 2026).
Flow: not observed for the same reason (OpenHands local app, "Loading..." screen observed on 5 September 2026).

## Job 4: Review the result

The overview panel contains a unified "Commits" drawer for commits and uncommitted changes, with actions for commit, pull, push, and pull request creation ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)).
The "Files" drawer provides a persistent tree, file tabs, and inline Markdown preview, while "View" opens a full file ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations); [conversation documentation capture](shots/openhands-docs-conversations.png)).
Flow ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)):

1. Open the overview panel.
2. Inspect files in "Files".
3. Review commits and uncommitted work in "Commits".
4. Commit, pull, push, or open a pull request from that drawer.

## Job 5: Run many agents at once

An agent can create a child conversation with `launch_child_conversation`, and local children can use either isolated worktrees or a shared workspace ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)).
The conversation list supports pinning, tags, and filters for all conversations, hidden automation runs, or only automation runs ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations); [conversation documentation capture](shots/openhands-docs-conversations.png)).
The active status vocabulary observed in the documentation is the current action label or "Thinking", but a dedicated "needs you" status was not observed ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)).
Flow ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)):

1. Launch a child conversation.
2. Choose isolation or a shared local workspace.
3. Return to the conversation list.
4. Filter, tag, or pin the runs and read each live activity chip.

## Job 6: Files and preview

The documented workspace places chat beside an embedded "VS Code" editor, a shared "Terminal Tab", an interactive "App Tab" for the running web server, and a non-interactive "Browser Tab" used by the agent ([Key Features](https://docs.openhands.dev/openhands/usage/key-features); [key features capture](shots/openhands-docs-features.png)).
The current Agent Canvas documentation describes "Files" as a focused drawer with tabs and a resizable persistent tree inside the conversation workspace ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)).
Flow ([Key Features](https://docs.openhands.dev/openhands/usage/key-features)):

1. Keep the request in chat.
2. Open the file in "VS Code" or "Files".
3. Run commands in "Terminal Tab".
4. Exercise the running product in "App Tab" without leaving the workspace.

## Job 7: Work from a phone

This job was observed only in documentation, not on a physical phone or tablet ([Mobile Access](https://docs.openhands.dev/openhands/usage/agent-canvas/mobile-access); [mobile access capture](shots/openhands-docs-mobile-access.png)).
The documented paths expose the same Agent Canvas through Tailscale or ngrok, with Tailscale using the machine address and port `8000` and ngrok requiring public mode plus a strong backend key ([Mobile Access](https://docs.openhands.dev/openhands/usage/agent-canvas/mobile-access)).
The documentation does not identify a reduced mobile feature set, so what is cut was not observed ([Mobile Access](https://docs.openhands.dev/openhands/usage/agent-canvas/mobile-access)).
Flow ([Mobile Access](https://docs.openhands.dev/openhands/usage/agent-canvas/mobile-access)):

1. Start Agent Canvas on the host.
2. Connect the devices with Tailscale or create an ngrok tunnel.
3. Open the documented address on the phone or tablet.
4. Use the browser interface.

## Job 8: Persistence and resume

The backend owns persistent conversation state, and the UI can reconnect to local, Docker, virtual-machine, or cloud backends ([Agent Server Overview](https://docs.openhands.dev/openhands/usage/agent-canvas/overview)).
Archiving hides a conversation without deleting backend history, but archive state is stored per backend in browser local storage and does not synchronize across machines ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)).
Branching from a message creates a separate conversation that preserves context and state while retaining the original in the sidebar ([Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)).
Flow ([Agent Server Overview](https://docs.openhands.dev/openhands/usage/agent-canvas/overview); [Agent Canvas Conversations](https://docs.openhands.dev/openhands/usage/agent-canvas/conversations)):

1. Reconnect the UI to the backend.
2. Select the conversation from the list.
3. Continue it or branch from a prior message.
4. Archive it when it should leave the active list.

## Three things it does better than our mockup

1. OpenHands makes the repository-versus-scratch decision explicit before the first conversation, while Surya's "New ask" begins with workspace, model, and mode fields inside one form ([OpenHands live start screen](shots/openhands-live-onboarding.png); [Surya new ask](../../mockup/shots/new-ask-desktop.png)).
2. OpenHands gives each conversation a current-action or "Thinking" chip and adds pin, tag, and automation filters, while Surya's agent list exposes status summaries but no comparable filtering controls ([conversation documentation capture](shots/openhands-docs-conversations.png); [Surya agents](../../mockup/shots/agents-desktop.png)).
3. OpenHands documents chat, editor, terminal, and interactive app as one workspace, while Surya currently separates code and the running app into top-level "Files" and "Preview" routes ([key features capture](shots/openhands-docs-features.png); [Surya files](../../mockup/shots/files-desktop.png); [Surya preview](../../mockup/shots/preview-desktop.png)).
