# Product

Factory

## What it is

Factory provides Droid through a visual app, a CLI, web and mobile review surfaces, and managed remote computers.
Factory Missions adds an orchestrator, workers, validators, and a Mission Control dashboard for larger projects.
Source: "Factory App", https://docs.factory.ai/docs/factory-app/overview, and "Running in the Factory App", https://docs.factory.ai/docs/missions/running-app.

## Job 1 - Start work from one sentence

Flow: 1. Select a project directory. 2. Select a model. 3. Choose "Normal Mode", "Spec Mode", or "Mission Mode". 4. Describe the outcome. 5. In Spec Mode, answer any clarifying questions and review the proposed plan. 6. Approve implementation or keep iterating.
The quickstart recommends first asking Droid to map the project, then asking for one small, verifiable change and requesting the diff before it applies anything.
"Normal Mode" is the default, while "Spec Mode" is read-only until Droid calls "ExitSpecMode" and asks for approval.
After a plan, the choices are to proceed with manual approvals, proceed at Low, Medium, or High autonomy, or keep iterating on the spec.
Source: "Factory App Quickstart", https://docs.factory.ai/docs/factory-app/quickstart, screenshot `docs/research/shots/factory-first-session.png`, and "Interaction Modes", https://docs.factory.ai/docs/autonomy-and-safety/specification-mode.

## Job 2 - Watch an agent work

Flow: 1. Select a session from the project-grouped sidebar. 2. Read the live transcript. 3. Expand grouped tool activity when detail matters. 4. Open a worker for terminal output, thought process, and subtasks.
The documented app example uses "Droid is thinking..." for an active session and collapses related work into summaries such as "codebase / search (12)" and "proto / generate stubs (4)" before showing individual Skill, file, and shell operations.
Mission Control adds a live progress log and lets a user inspect each worker's detailed terminal output, thought process, and completed subtasks.
Source: "Factory App Quickstart", https://docs.factory.ai/docs/factory-app/quickstart, screenshot `docs/research/shots/factory-first-session.png`, and "Running in the Factory App", https://docs.factory.ai/docs/missions/running-app, screenshot `docs/research/shots/factory-mission-control.png`.

## Job 3 - Answer a question or permission

Flow: 1. Droid reaches an action above the current autonomy level. 2. The run asks before continuing. 3. Approve this action or choose an "always allow" option. 4. Droid continues, and a reconnect preserves any still-pending question or approval.
Spec approval uses the explicit action "Proceed with implementation" and can retain manual approvals or raise the implementation autonomy level.
The app changelog states that pending questions and approvals remain available after a session reconnects.
A live question or permission card was not operated during this public-docs observation.
Source: "Autonomy Level", https://docs.factory.ai/docs/autonomy-and-safety/auto-run, screenshot `docs/research/shots/factory-approvals.png`, and "Full Changelog", https://docs.factory.ai/docs/changelog/release-notes#v0.205.0, screenshot `docs/research/shots/factory-pending-requests.png`.

## Job 4 - Review the result

Flow: 1. Keep the producing session open. 2. Preview its document, live site, or full code diff beside the session. 3. Comment on an element or diff line. 4. Let Droid act on the feedback in place. 5. Merge from the app or terminal.
Factory describes preview, feedback, and review as one connected workspace rather than separate handoff screens.
The docs state that live websites render beside the session, but launching and interacting with a generated site was not observed.
Source: "Factory App", https://docs.factory.ai/docs/factory-app/overview, screenshot `docs/research/shots/factory-workspace.png`, and "Welcome to Factory", https://docs.factory.ai/.

## Job 5 - Run many agents at once

Flow: 1. Start "Mission Mode". 2. Watch overall time, credits, and milestone progress in Mission Control. 3. Scan orchestrator, worker, and validator sessions. 4. Inspect a worker or feature. 5. Pause, redirect, re-plan, and resume when needed.
The left sidebar lists active and completed workers, the main view summarizes the active milestone, and the right sidebar carries model controls, feature progress, and a live progress log.
The observed completion label in the official Mission Control image was "COMPLETED".
The public material did not expose a distinct "needs you" label for a Mission.
Source: "Running in the Factory App", https://docs.factory.ai/docs/missions/running-app, screenshot `docs/research/shots/factory-mission-control.png`.

## Job 6 - Files and preview

Flow: 1. Leave the transcript in view. 2. Open the output beside it. 3. Review a document, spreadsheet, PDF, live site, or code diff. 4. Comment on the exact element or diff line. 5. Let Droid revise it in place.
The app works directly on the local filesystem and offers an output preview and diff surface next to the session.
A separate general-purpose editor was not documented on this surface.
Source: "Factory App", https://docs.factory.ai/docs/factory-app/overview, screenshot `docs/research/shots/factory-workspace.png`.

## Job 7 - Work from a phone

Flow: 1. Start work on the workstation. 2. Open the same synchronized session in a browser or on mobile. 3. Review and continue the session against the same Factory runtime.
Factory states that sessions carry the same project context across desktop, web, and mobile and that its quickstart covers the same session flow on all three surfaces.
The public docs did not identify which controls or views are cut on mobile.
Source: "Factory App Quickstart", https://docs.factory.ai/docs/factory-app/quickstart, screenshot `docs/research/shots/factory-first-session.png`, and "Welcome to Factory", https://docs.factory.ai/.

## Job 8 - Persistence and resume

Flow: 1. Find a past session or mission in the sidebar or Mission Control. 2. Reopen it after a reconnect or device change. 3. Restore its pending requests and current state. 4. Resume execution where it stopped.
The app can filter and group sessions across projects and machines, and synchronized sessions are available on desktop, web, and mobile.
Mission Control says a past mission resumes "exactly where you left off", while persistent worktrees can contain multiple sessions and remain available as projects.
Source: "Factory App", https://docs.factory.ai/docs/factory-app/overview, "Running in the Factory App", https://docs.factory.ai/docs/missions/running-app, screenshot `docs/research/shots/factory-mission-control.png`, and "Worktrees in the Factory App", https://docs.factory.ai/docs/factory-app/worktrees.

## Three things it does better than our mockup

1. Mission Control exposes orchestrator, worker, and validator roles with shared milestone progress and a live log, while the mockup's agents screen is organized as individual rows and nested children.
Evidence: `docs/research/shots/factory-mission-control.png` and `mockup/src/screens/agents.tsx`.
2. Factory keeps generated live sites, documents, diffs, and point-specific feedback beside the producing session, while the mockup divides feed, preview, files, and result into separate routes.
Evidence: `docs/research/shots/factory-workspace.png` and `mockup/src/screens/agent-feed.tsx`, `mockup/src/screens/preview.tsx`, `mockup/src/screens/files.tsx`, and `mockup/src/screens/result.tsx`.
3. Factory makes planning and execution two explicit stages with an approval boundary and an autonomy choice, while the mockup starts from a mode selector but does not expose a reviewable plan artifact in the new-ask screen.
Evidence: `docs/research/shots/factory-approvals.png` and `mockup/src/screens/new-ask.tsx`.
