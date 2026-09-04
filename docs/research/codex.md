# Product

Codex was observed as the "Codex" mode inside the ChatGPT Windows desktop app.
The observed document identified itself as "Codex" at `app://-/index.html`.
Source: Codex app screen, `docs/research/shots/codex-home.png`.

## Job 1 - Start work from one sentence

Flow: 1. Open "Codex" mode. 2. Read "What should we build?" 3. Optionally choose one of the four starter cards. 4. Use "Choose project" and the "Do anything" composer.
The four visible starters were "Explore and understand code", "Build a new feature, app, or tool", "Review code and suggest changes", and "Fix issues and failures".
The composer exposed the permission default "Approve for me" and the model default "5.6 Sol High".
No new ask was sent, so follow-up questions after the first sentence were not observed.
Source: Codex home screen, `docs/research/shots/codex-home.png`.

## Job 2 - Watch an agent work

Flow: 1. Open the existing "Review PR 9" session from "Recents". 2. Read progress prose in the main feed. 3. Expand "Worked for 4m 12s" to reveal the summarized work. 4. Read the final result below it.
The feed alternated compact user messages with plain-language progress and a final answer.
Integration noise was collapsed into "Used GitHub integration".
The expanded work disclosure showed summarized investigation and verification prose rather than a raw command transcript.
Source: Codex "Review PR 9" app screen, `docs/research/shots/codex-session.png`.

## Job 3 - Answer a question or permission

Flow: run requests permission or an answer -> not observed -> waiting behavior not observed.
The resting composer showed "Approve for me" and a separate "Change permissions" control, but no live permission card or question was present.
The controls were not opened because the research rules prohibited changing account or run state.
Source: Codex "Review PR 9" app screen, `docs/research/shots/codex-session.png`.

## Job 4 - Review the result

Flow: 1. Read generated results inline in the conversation. 2. Open "Changes +14,479 -0". 3. Review a file list and unified diff beside the conversation. 4. Continue with "Commit or push" or "Compare branch".
The image-generation result appeared as galleries with "Canvas" and "Next images" controls in the same session.
Opening "Changes +14,479 -0" replaced the compact environment card with a full right-hand review panel containing branch context, files, line numbers, and additions.
Running the changed application was not observed.
Source: Codex "Generate eligibility button" app screen, `docs/research/shots/codex-result.png` and `docs/research/shots/codex-diff.png`.

## Job 5 - Run many agents at once

Flow: 1. Open "View activity". 2. Read the "Priority" area. 3. See "Nothing needs attention" when the attention queue is empty.
The activity view replaced the long recent-session list with a deliberately sparse attention view.
No simultaneous active agents, grouping scheme, or running status words were present during observation.
The only observed needs-attention wording was "Priority" followed by "Nothing needs attention".
Source: Codex activity app screen, `docs/research/shots/codex-activity.png`.

## Job 6 - Files and preview

Flow: 1. Keep the conversation and generated visual result on the left. 2. Open "Changes". 3. Select files from the right-hand list. 4. Read the selected diff without leaving the session.
A read-only diff viewer was observed, but a general-purpose editor was not observed.
Generated images served as inline visual output, while changed files and code review occupied a neighboring panel.
A live application preview was not observed.
Source: Codex "Generate eligibility button" app screen, `docs/research/shots/codex-diff.png`.

## Job 7 - Work from a phone

Flow: Windows desktop app -> phone surface not observed.
No phone or narrow mobile interface was used during this desktop-only observation.
What Codex cuts on a phone was therefore not observed.
Source: Codex Windows app screen, `docs/research/shots/codex-home.png`.

## Job 8 - Persistence and resume

Flow: 1. Open "Recents". 2. Select the existing "Review PR 9" session. 3. Resume at its retained transcript, work disclosure, final result, project, branch, and sources.
The persistent session list remained visible beside both the home screen and an opened transcript.
Disconnect behavior was not observed.
Source: Codex home and resumed-session app screens, `docs/research/shots/codex-home.png` and `docs/research/shots/codex-session.png`.

## Three things it does better than our mockup

1. Codex compresses an idle multi-run view to "Priority" and "Nothing needs attention", which is calmer than the mockup's simultaneous needs-you and workspace summaries.
Evidence: `docs/research/shots/codex-activity.png`.
Mockup source: `mockup/src/screens/home.tsx`.
2. Codex keeps the conversation, generated visual result, file list, and diff in one split review surface, while the mockup separates these concerns across feed, result, and files routes.
Evidence: `docs/research/shots/codex-diff.png`.
Mockup source: `mockup/src/screens/agent-feed.tsx`, `mockup/src/screens/result.tsx`, and `mockup/src/screens/files.tsx`.
3. Codex collapses a run into the single disclosure "Worked for 4m 12s" and reveals a readable work summary on demand, while the mockup gives every feed event its own row.
Evidence: `docs/research/shots/codex-session.png`.
Mockup source: `mockup/src/screens/agent-feed.tsx` and `mockup/src/screens/agent-feed/events.tsx`.
