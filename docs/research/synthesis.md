# Synthesis: what ten shipped ADEs do, and what surya must change

Written 2026-09-05 from `docs/research/*.md`.
Four Codex researchers covered twelve products and captured 55 screenshots.
Every claim below traces to a product file in this directory, which traces to a page or a screen the researcher read.

## What was and was not covered

| Tier | Products | Result |
|---|---|---|
| Closed, docs and public material | Cursor, Windsurf, Devin, Factory, Kiro, GitHub Copilot coding agent | six files, all eight jobs answered or explicitly marked not observed |
| Open source, run or documented | OpenHands, Cline, Conductor | three files, three jobs not observed |
| Owner's Windows desktop | Codex, Claude Code desktop, Orca | Codex only |

Two gaps to name plainly.
Claude Code desktop and Orca were not observed: the researcher found only ChatGPT, Chrome, Snipping Tool and luvus open on that desktop, and the seat rules forbid launching an app on the owner's accounts.
Re-running that seat is cheap if the owner opens those two apps and says so.

One sourcing note: the Windsurf file cites `devin.ai` and `docs.devin.ai` throughout.
That is correct rather than confused.
Its own first line records that "The former Windsurf site now redirects to Devin Desktop", so the Windsurf findings are about the renamed product.

## The finding that matters most

**Eight of the ten products have no "needs you" queue at all.**

The researchers looked for one in each product and wrote down what they found:

| Product | What was found | Source |
|---|---|---|
| Cursor | "The docs do not name a dedicated 'needs you' list, so that exact status was not observed." | `cursor.md` job 5 |
| Devin | "did not show a dedicated visual 'needs you' status for the managed-child list" | `devin.md` job 5 |
| Factory | "did not expose a distinct 'needs you' label for a Mission" | `factory.md` job 5 |
| Kiro | "did not expose a distinct 'needs you' label" | `kiro.md` job 5 |
| OpenHands | "a dedicated 'needs you' status was not observed" | `openhands.md` job 5 |
| Cline | "does not show a specific 'needs you' queue or status term" | `cline.md` job 5 |
| Conductor | "A specific 'needs you' label or queue was not observed" | `conductor.md` job 5 |
| GitHub Copilot | "indicates when Copilot needs input", exact badge not observed | `github-copilot-coding-agent.md` job 5 |
| Windsurf | OS notification fires when a session "needs your input" | `windsurf.md` job 5 |
| Codex | "Priority" area, and "Nothing needs attention" when it is empty | `codex.md` job 5 |

Surya already has the best version of this in the field: a first-class inbox with the question, the command, and Approve and Reject in one card.
That is the product, and the redesign must make it the spine rather than one card among several on Home.

## The patterns most products share, per job

**Job 1, start from one sentence.** The strong products do not go straight from a sentence to code. They produce a plan you can read and approve. Kiro turns a description into "requirements, design, and tasks artifacts with explicit task states" (`kiro.md`). Windsurf's Plan mode "creates a durable implementation artifact with explicit clarification choices and an 'Implement' transition" (`windsurf.md`). Cline splits "discussion-only 'Plan' from mutating 'Act'" (`cline.md`). Factory makes "planning and execution two explicit stages with an approval boundary" (`factory.md`). Surya's New ask has a mode selector and no plan artifact.

**Job 2, watch it work.** The transcript is collapsed by default. Codex "collapses a run into the single disclosure 'Worked for 4m 12s'" (`codex.md`). Cursor puts "automatic checkpoints before significant changes" in the timeline (`cursor.md`). Devin connects "every shell command, code edit, and browser action to a navigable session timeline, giving auditability without forcing all detail into the chat feed" (`devin.md`). Surya gives every event its own row.

**Job 3, answer a question.** Answering once teaches the system. Cline "exposes approval policy by capability, including separate read, edit, command, browser, and MCP controls" (`cline.md`). Kiro "lets one permission decision become a precise rule by pattern and scope" (`kiro.md`). Surya answers each permission on its own, every time.

**Job 4, review the result.** One surface, not four. Codex "keeps the conversation, generated visual result, file list, and diff in one split review surface" (`codex.md`). Conductor's review "combines the changed-file list, commit filtering, line comments, agent feedback, and pull-request creation" (`conductor.md`). Factory "keeps generated live sites, documents, diffs, and point-specific feedback beside the producing session" (`factory.md`). Surya splits this across Feed, Files, Preview and Result as four top-level routes. Four of the researchers named this independently.

**Job 5, many agents.** The board is grouped by operational state, not by location. Windsurf's Agent Command Center uses "the exact columns 'Running,' 'Waiting for review,' and 'Done'" (`windsurf.md`). Cursor groups "under the exact headings 'IN PROGRESS' and 'READY FOR REVIEW'" (`cursor.md`). Cline and Conductor both run a Kanban of isolated worktrees. Surya groups by server and workspace, which answers "where" when the user is asking "what now".

**Job 6, files and preview.** They live inside the session. OpenHands "documents chat, editor, terminal, and interactive app as one workspace" (`openhands.md`). Windsurf's preview is active, not passive: "'Send element' turns a visual selection or console error into structured prompt context beside the agent" (`windsurf.md`).

**Job 7, the phone.** A deliberate reduction, not a shrunken desktop. Cursor's mobile app "deliberately omits the full editor, terminal, and file browser, showing changed files through the diff instead", and leaves configuration on the web (`cursor.md`). GitHub Mobile "reduces agent work to a clear task list with 'Open' and 'Merged' filters" (`github-copilot-coding-agent.md`). Surya renders all twelve surfaces at 390px.

**Job 8, resume.** The run survives the client. Cursor's cloud agents "continue working when the local machine or phone is disconnected", and Remote Control keeps tool calls on the user's machine while the loop moves to the cloud (`cursor.md`). Kiro makes "the same cloud session attachable from IDE, CLI, Web, and Mobile while the sandbox continues without a client" (`kiro.md`).

## The three flows in our mockup that must change

**1. Home answers the wrong question.**
Today Home is a greeting, a Needs you panel, then one card per workspace grouped by server.
That is an inventory of where things live.
Every product that has solved this groups by state instead: Running, Waiting for review, Done.
Change: Home becomes the attention queue. What needs you, at full weight and first. Then what is running. Then what is finished and unshipped. Workspace and server become filters and labels on the rows, not the top-level grouping. Codex's empty state, "Nothing needs attention", is the model for the quiet case.

**2. Review is spread across four routes.**
Feed, Files, Preview and Result are four separate top-level surfaces in `mockup/src/routes.tsx`.
Every product that reviews well fuses them into one session surface with panes.
Change: the agent session becomes the container. Transcript, changed files, diff and live preview become panes of one screen, not four destinations. Result stays a distinct surface only for the ship decision.

**3. Permission is answered, never learned.**
The inbox card asks "Run the database migration?" and offers Approve and Reject, and the next identical request asks again.
Cline and Kiro both turn one answer into a rule.
Change: every permission card gains a second, quieter action next to Approve, in the shape of "Always allow migrations in project-jag". One extra control, and the inbox gets shorter every week instead of longer.

## Per-screen layout brief, for the round that follows

The Variance Mandate still applies: adjacent surfaces must not share a layout family.

| Screen | Change | Because |
|---|---|---|
| Home | Rebuild as the attention queue: Needs you at full weight, then Running, then Done and unshipped. Workspace becomes a label. Quiet state says nothing needs attention. | `cursor.md`, `windsurf.md`, `codex.md` job 5 |
| New ask | One sentence in, a readable plan out, with an explicit Start button on the plan rather than on the form. | `kiro.md`, `windsurf.md`, `cline.md`, `factory.md` job 1 |
| Needs you | Keep. It is the product's strongest idea. Add the "always allow" rule action, and keep the first card open at full weight with the rest condensed. | this file, "the finding that matters most" |
| Agents | Group by state first, workspace second. Keep the spawned-agent tree, add the isolation cue Conductor documents. | `windsurf.md`, `conductor.md` job 5 |
| Agent feed | Collapse the run by default into one summary line that opens. Keep permission cards expanded and never collapsed. | `codex.md`, `devin.md`, `cursor.md` job 2 |
| Preview | Make it active: selecting an element or a console error should compose a message to the agent. | `windsurf.md` job 6 |
| Result | Keep the two-column shape. It is already the strongest screen. Add the test report next to the ship panel. | `devin.md` job 4 |
| Files | Fold into the session surface as a pane rather than a top-level route. | `openhands.md`, `conductor.md`, `factory.md` job 6 |
| Tasks | Weight the columns by count instead of four equal wells. | baseline critique, finding 2 |
| Messages | Unchanged this round. | no research signal |
| Settings | Add the approval-policy page the "always allow" rules need. | `cline.md`, `kiro.md` job 3 |
| Phone | Decide what to cut rather than shrinking all twelve. Supervision, answering and review stay. The editor and the file tree go. | `cursor.md`, `github-copilot-coding-agent.md` job 7 |
