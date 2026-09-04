# Product

## Devin

## What it is

Devin is a cloud software-engineering agent that writes, runs, and tests code in its own development environment. [Introducing Devin](https://docs.devin.ai/get-started/devin-intro) [devin-intro.png](shots/devin-intro.png)
Its web workspace combines a conversational session, progress log, Shell, IDE, Interactive Browser, pull-request review, and managed parallel sessions. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools) [Advanced Capabilities](https://docs.devin.ai/work-with-devin/advanced-capabilities)

## Job 1 - Start work from one sentence

The new-session page offers "Ask" and "Agent" modes, and Devin recommends beginning in Ask unless the user already has a fully scoped plan. [Your First Session](https://docs.devin.ai/get-started/first-run) [devin-first-session.png](shots/devin-first-session.png)
Agent mode then asks for a repository and an agent configuration, with "Devin" documented as the default general-purpose choice. [Your First Session](https://docs.devin.ai/get-started/first-run)
The public product shot shows a concise request at the top of the session and an attached test report beside the transcript. [Devin homepage](https://devin.ai/) [devin-home-product.png](shots/devin-home-product.png)

1. Enter the task in "Ask" to explore and create a scoped prompt, or choose "Agent" for already-scoped work. [Your First Session](https://docs.devin.ai/get-started/first-run)
2. Select the repository and agent configuration, with the general-purpose "Devin" as the documented default. [Your First Session](https://docs.devin.ai/get-started/first-run)
3. Select "Send to Devin" from an Ask plan or send the Agent-mode prompt directly. [Your First Session](https://docs.devin.ai/get-started/first-run)

## Job 2 - Watch an agent work

The "Progress" tab unifies shell commands, code edits, and browser activity, and any progress step opens its detailed work at that point in the session. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools) [devin-session-tools.png](shots/devin-session-tools.png)
Shell history shows every command, output previews, copy actions, and time navigation, while future commands in the recorded timeline appear greyed out. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)
"Side chats" answer read-only questions beside the worklog without interrupting the main run, but the public docs did not state that hidden model reasoning is exposed. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)

1. Start the session and watch its named progress steps. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)
2. Select a step to inspect linked shell, code, and browser activity in one timeline. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)
3. Open a "Side chat" for a question while the main session continues running. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)

## Job 3 - Answer a question or permission

During testing, Devin requests missing secrets up front and can require the user to complete authentication, MFA, or CAPTCHA work in the "Interactive Browser." [Testing and Video Recordings](https://docs.devin.ai/work-with-devin/testing-and-recordings) [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)
Taking over the IDE requires stopping the session first, after which the user can make changes and resume by telling Devin what changed. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)
The exact visual treatment of a generic question card was not observed in the public pages. [Your First Session](https://docs.devin.ai/get-started/first-run)

1. Devin asks for a missing secret or browser intervention when automated progress is blocked. [Testing and Video Recordings](https://docs.devin.ai/work-with-devin/testing-and-recordings)
2. The user supplies a secret or opens "Interactive Browser" and completes the blocked step. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)
3. The user resumes the agent after any manual IDE work and describes the changes made. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)

## Job 4 - Review the result

The session IDE supports real-time diff review and manual takeover, while the testing workflow can attach a processed end-to-end video after the pull request is created. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools) [Testing and Video Recordings](https://docs.devin.ai/work-with-devin/testing-and-recordings)
"Devin Review" groups related edits rather than ordering files alphabetically, detects moved code, supports codebase-aware chat, and offers pull-request actions including merge, close, draft, ready, and auto-merge. [Devin Review](https://docs.devin.ai/work-with-devin/devin-review) [devin-review.png](shots/devin-review.png)
The public homepage shows a finished session with separate pull-request cards and a side-by-side test report containing before and after evidence. [Devin homepage](https://devin.ai/) [devin-home-product.png](shots/devin-home-product.png)

1. Inspect the session diff in the IDE or open the orange "Review" action for its pull request. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools) [Devin Review](https://docs.devin.ai/work-with-devin/devin-review)
2. Review logically grouped changes, findings, checks, and the attached test evidence. [Devin Review](https://docs.devin.ai/work-with-devin/devin-review) [Testing and Video Recordings](https://docs.devin.ai/work-with-devin/testing-and-recordings)
3. Comment, approve, request changes, apply a suggested commit, or complete the pull-request workflow without leaving Devin Review. [Devin Review](https://docs.devin.ai/work-with-devin/devin-review)

## Job 5 - Run many agents at once

"Managed Devins" let one coordinator break down a large task, launch child sessions in isolated VMs, message them, monitor their compute use, put them to sleep, terminate them, resolve conflicts, and compile the result. [Advanced Capabilities](https://docs.devin.ai/work-with-devin/advanced-capabilities) [devin-managed-parallel.png](shots/devin-managed-parallel.png)
The product-owned homepage describes a "team of Devins" for multi-week, multi-repository work and shows recent sessions in a persistent left sidebar. [Devin homepage](https://devin.ai/) [devin-home-product.png](shots/devin-home-product.png)
The public docs exposed child controls but did not show a dedicated visual "needs you" status for the managed-child list. [Advanced Capabilities](https://docs.devin.ai/work-with-devin/advanced-capabilities)

1. Ask the coordinator to split the work or explicitly request one managed session per module or service. [Advanced Capabilities](https://docs.devin.ai/work-with-devin/advanced-capabilities)
2. Review the proposed child sessions and approve their launch. [Advanced Capabilities](https://docs.devin.ai/work-with-devin/advanced-capabilities)
3. Let the coordinator monitor, message, pause, stop, reconcile, and combine the child results. [Advanced Capabilities](https://docs.devin.ai/work-with-devin/advanced-capabilities)

## Job 6 - Files and preview

The embedded IDE supports familiar file navigation and real-time diff review, while the Shell exposes commands and outputs from the same session. [Introducing Devin](https://docs.devin.ai/get-started/devin-intro) [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)
The "Desktop" tab, previously named "Browser," provides the interactive browser and full desktop environment for local-app testing, visual checks, authentication, screenshots, and recordings. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)

1. Open a progress step and inspect its files or diff in the embedded IDE. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)
2. Switch to "Desktop" to watch or take over the browser-based test. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)
3. Return to the unified progress timeline, where the related browser, shell, and edit events remain connected. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)

## Job 7 - Work from a phone

Not observed because the authenticated session workspace redirected to the public marketing site without a login, and the product docs did not document a mobile agent controller. [Devin homepage](https://devin.ai/)
The marketing site has a responsive phone layout, but that does not establish whether the Shell, IDE, Browser, review, or managed-agent controls are usable from a phone. [Devin homepage](https://devin.ai/)

1. The public mobile marketing page can be opened, but it is not the agent workspace. [Devin homepage](https://devin.ai/)
2. Starting, supervising, and reviewing a real session on a phone were not observed. [Devin homepage](https://devin.ai/)
3. No conclusion about what the authenticated product cuts on mobile is supported by the available evidence. [Devin homepage](https://devin.ai/)

## Job 8 - Persistence and resume

The product homepage shows a persistent "Recent" session list, and the docs let users @ mention prior sessions into a new prompt for context. [Devin homepage](https://devin.ai/) [Your First Session](https://docs.devin.ai/get-started/first-run) [devin-home-product.png](shots/devin-home-product.png)
The IDE takeover flow explicitly stops Devin, permits manual edits, and then resumes the same work after the user explains what changed. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)
Managed coordinators can put child sessions to sleep and schedule messages to themselves to revisit long-running work, while dynamic workflows are described as recorded and resumable. [Advanced Capabilities](https://docs.devin.ai/work-with-devin/advanced-capabilities)
Behavior after an unplanned network disconnect was not observed in the public documentation. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)

1. Reopen a prior session from the recent-session sidebar or reference it with "@Sessions." [Devin homepage](https://devin.ai/) [Your First Session](https://docs.devin.ai/get-started/first-run)
2. Resume an intentionally stopped session after describing any manual edits, or wake a managed child that the coordinator put to sleep. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools) [Advanced Capabilities](https://docs.devin.ai/work-with-devin/advanced-capabilities)
3. Continue from the retained transcript, progress events, files, and pull-request context. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools)

## Three things it does better than our mockup

1. Devin's finished-session view makes the result the focal point by pairing the transcript with pull-request cards and a concrete test report instead of ending on a generic summary card. [Devin homepage](https://devin.ai/) [devin-home-product.png](shots/devin-home-product.png)
2. Its progress model connects every shell command, code edit, and browser action to a navigable session timeline, giving auditability without forcing all detail into the chat feed. [Devin Session Tools](https://docs.devin.ai/work-with-devin/devin-session-tools) [devin-session-tools.png](shots/devin-session-tools.png)
3. Its many-agent workflow has an explicit coordinator that proposes child sessions, controls their lifecycle, resolves conflicts, and compiles output, which is operationally deeper than a flat board of independent cards. [Advanced Capabilities](https://docs.devin.ai/work-with-devin/advanced-capabilities) [devin-managed-parallel.png](shots/devin-managed-parallel.png)
