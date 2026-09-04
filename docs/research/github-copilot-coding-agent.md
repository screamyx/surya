# Product

GitHub Copilot cloud agent, formerly documented as GitHub Copilot coding agent

## What it is

Copilot cloud agent researches a GitHub repository, plans a change, works on a branch in the background, and hands the result back through a session and pull request.
GitHub exposes the same session workflow from GitHub.com, GitHub Mobile, IDEs, the CLI, APIs, and connected work trackers.
Source: "About GitHub Copilot cloud agent", https://docs.github.com/en/copilot/concepts/agents/cloud-agent/about-cloud-agent, and "Starting GitHub Copilot sessions", https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/start-copilot-sessions.

## Job 1 - Start work from one sentence

Flow: 1. Open the agents tab, agents panel, dashboard task box, or an issue. 2. Select a repository. 3. Type one prompt. 4. Optionally choose a base branch, agent, model, and reasoning level. 5. Submit. 6. Open the resulting session or pull request.
The shared form is called "New agent task", and the example prompt is "Implement a user friendly message for common errors."
Asking for a pull request in the prompt makes that handoff explicit, while assigning an issue makes Copilot raise a pull request and request the user's review when finished.
If repository and branch are omitted during issue assignment, Copilot uses the issue repository and its default branch.
Source: "Using Copilot cloud agent on GitHub", https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/use-cloud-agent-on-github, screenshot `docs/research/shots/github-copilot-new-agent-task.png`.

## Job 2 - Watch an agent work

Flow: 1. Open the global agents panel or agents page. 2. Select a session. 3. Read the overview and session log. 4. Watch progress, token usage, and session length. 5. Steer from the prompt box after the current tool call ends.
Session logs explicitly show Copilot's internal reasoning and the tools used to understand the repository, make changes, and validate the work.
The ephemeral development environment can run tests and linters before Copilot pushes changes.
The public docs did not describe which log rows collapse by default.
Source: "Managing agent sessions", https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents, screenshot `docs/research/shots/github-copilot-session-logs.png`.

## Job 3 - Answer a question or permission

Flow: 1. A low-confidence issue automation suggestion waits in the issue's approvals panel. 2. Inspect its proposed change, rationale, and confidence. 3. Select "Accept", "Decline", "Accept all", or "Decline all". 4. Accepted changes apply immediately, while declined suggestions leave the issue unchanged.
For code output, GitHub Actions workflows pushed by Copilot wait in the pull request merge box until a user selects "Approve and run workflows", unless a repository administrator has allowed automatic runs.
A conversational question card from a coding session was not observed.
Source: "Managing rationale, confidence, and approvals for issues", https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/manage-rationale-confidence-approvals, screenshot `docs/research/shots/github-copilot-approvals-panel.png`, and "Review output from Copilot", https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/review-copilot-output, screenshot `docs/research/shots/github-copilot-review.png`.

## Job 4 - Review the result

Flow: 1. Open the pull request. 2. Review the overview, session rationale, and "Files changed". 3. Inspect workflow-file changes. 4. Select "Approve and run workflows" when safe. 5. Mention `@copilot` in a review comment for revisions. 6. Require another reviewer before merge when branch rules require approval.
GitHub warns that the task starter's approval of a Copilot pull request does not count toward required approvals, so another reviewer must approve it.
Copilot-authored commits are signed, appear as "Verified", and link back to the session logs for audit context.
Source: "Review output from Copilot", https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/review-copilot-output, screenshot `docs/research/shots/github-copilot-review.png`, and "Managing agent sessions", https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents.

## Job 5 - Run many agents at once

Flow: 1. Open the agents panel from any GitHub page or visit the agents page. 2. Scan sessions across repositories. 3. Select a task to see its overview, logs, and files. 4. Jump in when Copilot needs input. 5. Follow the quick link to its pull request.
GitHub's changelog calls this centralized surface a "mission control" and says it shows task status at a glance and indicates when Copilot needs input.
The official image shows multiple sessions together and the exact status "In progress".
The complete status vocabulary and a separate exact "needs you" badge were not observed.
Source: "A mission control to assign, steer, and track Copilot coding agent tasks", https://github.blog/changelog/2025-10-28-a-mission-control-to-assign-steer-and-track-copilot-coding-agent-tasks, screenshot `docs/research/shots/github-copilot-task-list.png`.

## Job 6 - Files and preview

Flow: 1. Open a session. 2. Keep session logs beside "Overview" and "Files changed". 3. Comment directly in "Files changed". 4. Continue in Codespaces, VS Code Insiders, or GitHub CLI when hands-on editing is needed.
GitHub's centralized task view relates the agent transcript, commit rationale, and diff without sending the reviewer to unrelated pages.
A built-in live application preview was not observed.
Source: "A mission control to assign, steer, and track Copilot coding agent tasks", https://github.blog/changelog/2025-10-28-a-mission-control-to-assign-steer-and-track-copilot-coding-agent-tasks, screenshot `docs/research/shots/github-copilot-task-list.png`.

## Job 7 - Work from a phone

Flow: 1. Open GitHub Mobile. 2. Tap "New Session". 3. Select a repository and enter the prompt. 4. Optionally select a branch, agent, model, and reasoning level. 5. Track generated pull requests under "Agent Tasks". 6. Filter them by "Open" or "Merged".
GitHub Mobile also lets a user assign an issue to Copilot from the issue's assignee editor.
The docs expose starting, issue assignment, and pull-request tracking, but do not state which desktop session-log or review controls are omitted on mobile.
Source: "Using Copilot cloud agent on GitHub Mobile", https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/use-cloud-agent-on-mobile, screenshot `docs/research/shots/github-copilot-mobile.png`.

## Job 8 - Persistence and resume

Flow: 1. Start the cloud task. 2. Let Copilot work in the background. 3. Reopen it from the agents panel or agents page. 4. Review retained logs, commits, and file changes. 5. Steer it or continue the session in an IDE or CLI. 6. Archive a stopped session when it no longer belongs in the active list.
Stopping preserves commits already pushed, cloud sessions can be archived but not deleted, and repository collaborators can view shared cloud sessions in the "All sessions" view.
The public docs did not state a disconnect timeout because the agent is presented as background cloud work rather than a client-bound process.
Source: "About GitHub Copilot cloud agent", https://docs.github.com/en/copilot/concepts/agents/cloud-agent/about-cloud-agent, and "Managing agent sessions", https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents, screenshot `docs/research/shots/github-copilot-session-logs.png`.

## Three things it does better than our mockup

1. GitHub makes the pull request the reviewable contract and requires an independent approver when repository rules demand approval, while the mockup's result screen displays review state without documenting this separation of duties.
Evidence: `docs/research/shots/github-copilot-review.png` and `mockup/src/screens/result.tsx`.
2. GitHub puts cross-repository sessions, task status, the needs-input cue, logs, overview, and files into one mission-control flow, while the mockup separates the home rollup, agents list, feed, and result.
Evidence: `docs/research/shots/github-copilot-task-list.png` and `mockup/src/screens/home.tsx`, `mockup/src/screens/agents.tsx`, `mockup/src/screens/agent-feed.tsx`, and `mockup/src/screens/result.tsx`.
3. GitHub Mobile reduces agent work to a clear task list with "Open" and "Merged" filters, while the mockup's mobile agent surfaces do not expose a comparable outcome-history filter.
Evidence: `docs/research/shots/github-copilot-mobile.png` and `mockup/src/screens/agents.tsx`.
