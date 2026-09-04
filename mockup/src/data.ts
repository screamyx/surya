// Shared fake data for the surya 1.0 RC mockup. Builders read from here, never edit.
export type AgentStatus = "working" | "needs-you" | "done" | "idle"

export type Workspace = {
  id: string
  name: string
  repo: string
  branch: string
  worktree: string
  previewUrl: string
  agents: Agent[]
}

export type Agent = {
  id: string
  workspaceId: string
  name: string
  model: string
  status: AgentStatus
  summary: string
  startedAt: string
  lastEventAt: string
  task?: string
  parentId?: string // the agent that spawned this one; undefined = you started it
  children: Agent[]
}

export type InboxKind = "permission" | "question" | "pin" | "result"
export type InboxItem = {
  id: string
  kind: InboxKind
  agentId: string
  workspaceId: string
  title: string
  body: string
  at: string
  // permission
  tool?: string
  command?: string
  // question
  options?: { label: string; description: string }[]
}

export type TaskStatus = "queued" | "running" | "blocked" | "done"
export type Task = {
  id: string
  workspaceId: string
  title: string
  status: TaskStatus
  agentId?: string
  deps: string[]
  source: "you" | "pin" | "agent"
  createdAt: string
}

export type Pin = {
  id: string
  workspaceId: string
  x: number // percent of the frame width
  y: number // percent of the frame height
  selector: string
  note: string
  by: "you" | "agent"
  status: "open" | "taken" | "fixed"
  taskId?: string
}

export type FileNode = {
  name: string
  path: string
  kind: "dir" | "file"
  touched?: "agent" | "you"
  children?: FileNode[]
}

export type FeedEvent =
  | { id: string; kind: "user"; at: string; text: string }
  | { id: string; kind: "assistant"; at: string; text: string }
  | { id: string; kind: "thinking"; at: string; text: string }
  | { id: string; kind: "tool"; at: string; tool: string; input: string; output: string; ms: number; ok: boolean }
  | { id: string; kind: "diff"; at: string; file: string; added: number; removed: number; hunk: string }
  | { id: string; kind: "card"; at: string; card: CardSample }
  | { id: string; kind: "subagent"; at: string; name: string; summary: string; events: number }
  | { id: string; kind: "permission"; at: string; tool: string; command: string; decided?: "approved" | "rejected" }
  | { id: string; kind: "question"; at: string; question: string; options: string[]; answered?: string }
  | { id: string; kind: "command"; at: string; name: string; args: string; source: SlashSource; description: string }

// Slash commands are whatever Claude Code reports at session start. Nothing here is surya's own.
export type SlashSource = "built-in" | "user skill" | "project skill" | "plugin"
export type SlashCommand = { name: string; description: string; source: SlashSource; args?: string }

export type CardSample =
  | { type: "vehicle"; stockNo: string; title: string; price: string; days: number; photo: string; status: string }
  | { type: "table"; title: string; columns: string[]; rows: string[][] }
  | { type: "form"; title: string; fields: { label: string; type: "text" | "select" | "date"; value?: string; options?: string[] }[]; submit: string }
  | { type: "approval"; title: string; summary: string; code: string }
  | { type: "diff-summary"; title: string; files: { path: string; added: number; removed: number }[]; note: string }
  | { type: "metric"; title: string; value: string; delta: string; series: number[] }

export const now = "2026-09-05T01:12:00+08:00"

export const workspaces: Workspace[] = [
  {
    id: "project-jag",
    name: "project-jag",
    repo: "screamyx/project-jag",
    branch: "v2",
    worktree: "/store/agent-worktrees/lead-follow-up",
    previewUrl: "https://pc-ajim.tail82fec1.ts.net:8457",
    agents: [],
  },
  {
    id: "surya",
    name: "surya",
    repo: "screamyx/surya",
    branch: "main",
    worktree: "/home/user/git/surya",
    previewUrl: "http://pc-ajim.tail82fec1.ts.net:4790",
    agents: [],
  },
  {
    id: "kss-marketing",
    name: "kss-marketing",
    repo: "screamyx/kss-marketing",
    branch: "main",
    worktree: "/home/user/git/kss-marketing",
    previewUrl: "https://videos.tail82fec1.ts.net",
    agents: [],
  },
]

export const agents: Agent[] = [
  {
    id: "raven",
    workspaceId: "project-jag",
    name: "raven",
    model: "claude-fable-5-1",
    status: "needs-you",
    summary: "Wants approval to run the DB migration for lead follow-up fields",
    startedAt: "2026-09-05T00:41:00+08:00",
    lastEventAt: "2026-09-05T01:10:12+08:00",
    task: "t-1",
    children: [],
  },
  {
    id: "raven-tests",
    workspaceId: "project-jag",
    name: "tests",
    model: "claude-sonnet-5",
    status: "working",
    summary: "Running the Pest suite for the follow-up migration, 12 of 14 green",
    startedAt: "2026-09-05T01:08:00+08:00",
    lastEventAt: "2026-09-05T01:11:50+08:00",
    parentId: "raven",
    children: [],
  },
  {
    id: "raven-review",
    workspaceId: "project-jag",
    name: "review",
    model: "gpt-5.6-sol",
    status: "done",
    summary: "Reviewed the migration and the model change, one nit left as a comment",
    startedAt: "2026-09-05T01:05:00+08:00",
    lastEventAt: "2026-09-05T01:09:30+08:00",
    parentId: "raven",
    children: [],
  },
  {
    id: "kite",
    workspaceId: "project-jag",
    name: "kite",
    model: "claude-opus-5",
    status: "working",
    summary: "Writing Pest tests for the follow-up reminder job, 3 of 5 passing",
    startedAt: "2026-09-05T00:52:00+08:00",
    lastEventAt: "2026-09-05T01:11:58+08:00",
    task: "t-2",
    children: [],
  },
  {
    id: "heron",
    workspaceId: "project-jag",
    name: "heron",
    model: "gpt-5.6-sol",
    status: "done",
    summary: "Fixed the tile jump on upload, PR #648 open, CI green",
    startedAt: "2026-09-04T23:20:00+08:00",
    lastEventAt: "2026-09-05T00:38:00+08:00",
    task: "t-4",
    children: [],
  },
  {
    id: "ibis",
    workspaceId: "surya",
    name: "ibis",
    model: "claude-fable-5-1",
    status: "working",
    summary: "Building the inbox screen, wiring the permission card",
    startedAt: "2026-09-05T01:02:00+08:00",
    lastEventAt: "2026-09-05T01:11:40+08:00",
    task: "t-6",
    children: [],
  },
  {
    id: "ibis-shots",
    workspaceId: "surya",
    name: "shots",
    model: "claude-haiku-4-5-20251001",
    status: "needs-you",
    summary: "Wants to install a Chrome build to take phone screenshots",
    startedAt: "2026-09-05T01:09:00+08:00",
    lastEventAt: "2026-09-05T01:11:20+08:00",
    parentId: "ibis",
    children: [],
  },
  {
    id: "wren",
    workspaceId: "surya",
    name: "wren",
    model: "claude-opus-5",
    status: "idle",
    summary: "Waiting for the next task",
    startedAt: "2026-09-05T00:58:00+08:00",
    lastEventAt: "2026-09-05T01:05:00+08:00",
    children: [],
  },
  {
    id: "finch",
    workspaceId: "kss-marketing",
    name: "finch",
    model: "claude-sonnet-5",
    status: "done",
    summary: "Rendered 42 catalog reels, all published",
    startedAt: "2026-09-04T22:00:00+08:00",
    lastEventAt: "2026-09-04T23:48:00+08:00",
    children: [],
  },
]
for (const a of agents) a.children = agents.filter((c) => c.parentId === a.id)
for (const w of workspaces) w.agents = agents.filter((a) => a.workspaceId === w.id && !a.parentId)

// Status of an agent including everything it spawned: the worst state wins.
const rank: Record<AgentStatus, number> = { "needs-you": 0, working: 1, done: 2, idle: 3 }
export const rollup = (a: Agent): AgentStatus =>
  [a, ...a.children.map((c) => ({ status: rollup(c) }))].reduce<AgentStatus>((worst, x) => (rank[x.status] < rank[worst] ? x.status : worst), a.status)
export const needsYouCount = (list: Agent[]): number => list.reduce((n, a) => n + (a.status === "needs-you" ? 1 : 0) + needsYouCount(a.children), 0)
export const byPriority = (list: Agent[]): Agent[] => [...list].sort((x, y) => rank[rollup(x)] - rank[rollup(y)])
export const flatten = (list: Agent[]): Agent[] => list.flatMap((a) => [a, ...flatten(a.children)])

export const inbox: InboxItem[] = [
  {
    id: "i-1",
    kind: "permission",
    agentId: "raven",
    workspaceId: "project-jag",
    title: "Run the database migration?",
    body: "Adds two columns to leads: next_follow_up_at and follow_up_note. Runs on the dev database only.",
    at: "2026-09-05T01:10:12+08:00",
    tool: "Bash",
    command: "docker compose exec -T app php artisan migrate --force",
  },
  {
    id: "i-2",
    kind: "question",
    agentId: "kite",
    workspaceId: "project-jag",
    title: "When should the reminder fire?",
    body: "The spec says 'the morning after'. I need a time.",
    at: "2026-09-05T01:04:30+08:00",
    options: [
      { label: "08:00 KL time", description: "Before the showroom opens" },
      { label: "09:30 KL time", description: "After the morning meeting" },
      { label: "Same time the lead came in", description: "24 hours after the first contact" },
    ],
  },
  {
    id: "i-3",
    kind: "pin",
    agentId: "ibis",
    workspaceId: "surya",
    title: "Pin on /inbox: 'approve button too small on phone'",
    body: "ibis picked this up and proposes a full-width button row under 640px. Accept?",
    at: "2026-09-05T00:59:00+08:00",
  },
  {
    id: "i-4",
    kind: "result",
    agentId: "heron",
    workspaceId: "project-jag",
    title: "Tile jump on upload is fixed",
    body: "PR #648 is open and CI is green. 3 files changed, +41 -12. Ready to ship.",
    at: "2026-09-05T00:38:00+08:00",
  },
]

export const tasks: Task[] = [
  { id: "t-1", workspaceId: "project-jag", title: "Add follow-up fields to leads", status: "running", agentId: "raven", deps: [], source: "you", createdAt: "2026-09-05T00:40:00+08:00" },
  { id: "t-2", workspaceId: "project-jag", title: "Reminder job the morning after a lead", status: "running", agentId: "kite", deps: ["t-1"], source: "you", createdAt: "2026-09-05T00:40:00+08:00" },
  { id: "t-3", workspaceId: "project-jag", title: "Show next follow-up on the lead card in the PWA", status: "blocked", deps: ["t-1", "t-2"], source: "you", createdAt: "2026-09-05T00:40:00+08:00" },
  { id: "t-4", workspaceId: "project-jag", title: "Tile jumps when a photo finishes uploading", status: "done", agentId: "heron", deps: [], source: "pin", createdAt: "2026-09-04T23:15:00+08:00" },
  { id: "t-5", workspaceId: "project-jag", title: "Media tab scrolls sideways after Library upload (#553)", status: "queued", deps: [], source: "you", createdAt: "2026-09-04T18:00:00+08:00" },
  { id: "t-6", workspaceId: "surya", title: "Inbox screen with permission and question cards", status: "running", agentId: "ibis", deps: [], source: "you", createdAt: "2026-09-05T01:00:00+08:00" },
  { id: "t-7", workspaceId: "surya", title: "Approve button full width on phone", status: "queued", deps: ["t-6"], source: "pin", createdAt: "2026-09-05T00:59:00+08:00" },
  { id: "t-8", workspaceId: "surya", title: "File tree lights up files the agent touches", status: "queued", deps: [], source: "agent", createdAt: "2026-09-05T00:45:00+08:00" },
]

export const pins: Pin[] = [
  { id: "p-1", workspaceId: "project-jag", x: 62, y: 38, selector: "button.upload-tile", note: "Tile jumps when the photo finishes. Should stay put.", by: "you", status: "fixed", taskId: "t-4" },
  { id: "p-2", workspaceId: "project-jag", x: 28, y: 71, selector: ".lead-card .follow-up", note: "Show the next follow-up date here, in red if overdue.", by: "you", status: "taken", taskId: "t-3" },
  { id: "p-3", workspaceId: "project-jag", x: 84, y: 14, selector: "header .sync-badge", note: "This badge says 'synced' while the upload is still running.", by: "you", status: "open" },
  { id: "p-4", workspaceId: "project-jag", x: 45, y: 52, selector: ".vehicle-grid", note: "I made the grid 3 across on tablet. Check it on yours?", by: "agent", status: "open" },
]

export const fileTree: FileNode[] = [
  {
    name: "backend", path: "backend", kind: "dir", children: [
      { name: "app", path: "backend/app", kind: "dir", children: [
        { name: "Models", path: "backend/app/Models", kind: "dir", children: [
          { name: "Lead.php", path: "backend/app/Models/Lead.php", kind: "file", touched: "agent" },
          { name: "Vehicle.php", path: "backend/app/Models/Vehicle.php", kind: "file" },
        ]},
        { name: "Jobs", path: "backend/app/Jobs", kind: "dir", children: [
          { name: "SendFollowUpReminder.php", path: "backend/app/Jobs/SendFollowUpReminder.php", kind: "file", touched: "agent" },
        ]},
      ]},
      { name: "database", path: "backend/database", kind: "dir", children: [
        { name: "migrations", path: "backend/database/migrations", kind: "dir", children: [
          { name: "2026_09_05_000001_add_follow_up_to_leads.php", path: "backend/database/migrations/2026_09_05_000001_add_follow_up_to_leads.php", kind: "file", touched: "agent" },
        ]},
      ]},
      { name: "tests", path: "backend/tests", kind: "dir", children: [
        { name: "Feature", path: "backend/tests/Feature", kind: "dir", children: [
          { name: "FollowUpReminderTest.php", path: "backend/tests/Feature/FollowUpReminderTest.php", kind: "file", touched: "agent" },
        ]},
      ]},
      { name: "staff-react", path: "backend/staff-react", kind: "dir", children: [
        { name: "src", path: "backend/staff-react/src", kind: "dir", children: [
          { name: "LeadCard.tsx", path: "backend/staff-react/src/LeadCard.tsx", kind: "file", touched: "you" },
        ]},
      ]},
    ],
  },
  { name: "specs", path: "specs", kind: "dir", children: [
    { name: "behaviors", path: "specs/behaviors", kind: "dir", children: [
      { name: "crm-lead-follow-up.md", path: "specs/behaviors/crm-lead-follow-up.md", kind: "file" },
    ]},
  ]},
  { name: "AGENTS.md", path: "AGENTS.md", kind: "file" },
]

export const fileContents: Record<string, string> = {
  "backend/app/Models/Lead.php": `<?php

namespace App\\Models;

use Illuminate\\Database\\Eloquent\\Model;

class Lead extends Model
{
    protected $fillable = [
        'customer_id',
        'vehicle_id',
        'source',
        'next_follow_up_at',
        'follow_up_note',
    ];

    protected function casts(): array
    {
        return [
            'next_follow_up_at' => 'datetime',
        ];
    }

    public function isOverdue(): bool
    {
        return $this->next_follow_up_at?->isPast() ?? false;
    }
}
`,
  "backend/app/Jobs/SendFollowUpReminder.php": `<?php

namespace App\\Jobs;

use App\\Models\\Lead;
use Illuminate\\Contracts\\Queue\\ShouldQueue;
use Illuminate\\Foundation\\Queue\\Queueable;

class SendFollowUpReminder implements ShouldQueue
{
    use Queueable;

    public function __construct(public Lead $lead) {}

    public function handle(): void
    {
        if ($this->lead->next_follow_up_at === null) {
            return;
        }

        // TODO: push to the salesperson's phone
    }
}
`,
  "specs/behaviors/crm-lead-follow-up.md": `# Behavior: lead follow-up

Every lead carries a next follow-up date and a one-line note.
The morning after a lead comes in, the salesperson gets one reminder.
`,
}

export const leadPhpBefore = `    protected $fillable = [
        'customer_id',
        'vehicle_id',
        'source',
    ];
`
export const leadPhpAfter = `    protected $fillable = [
        'customer_id',
        'vehicle_id',
        'source',
        'next_follow_up_at',
        'follow_up_note',
    ];

    protected function casts(): array
    {
        return [
            'next_follow_up_at' => 'datetime',
        ];
    }
`

export const cards: CardSample[] = [
  { type: "vehicle", stockNo: "KSS-0412", title: "2021 Toyota Alphard 2.5 SC", price: "RM 268,800", days: 34, photo: "https://picsum.photos/seed/alphard/640/400", status: "In stock" },
  { type: "table", title: "Leads with no follow-up date", columns: ["Lead", "Car", "Came in", "Salesperson"], rows: [["Ahmad F.", "Alphard SC", "2 days ago", "Zul"], ["Mei Ling", "Vellfire ZG", "4 days ago", "Farah"], ["Rajesh K.", "Harrier Z", "6 days ago", "Zul"]] },
  { type: "form", title: "Set the follow-up", fields: [{ label: "Lead", type: "text", value: "Ahmad F." }, { label: "When", type: "date", value: "2026-09-06" }, { label: "Salesperson", type: "select", value: "Zul", options: ["Zul", "Farah", "Hafiz"] }, { label: "Note", type: "text", value: "Ask about trade-in" }], submit: "Save follow-up" },
  { type: "approval", title: "Write 3 leads", summary: "Set next_follow_up_at to tomorrow 08:00 for the three leads above.", code: "Lead::whereIn('id', [412, 418, 421])\n    ->update(['next_follow_up_at' => now()->addDay()->setTime(8, 0)]);" },
  { type: "diff-summary", title: "What changed", files: [{ path: "backend/app/Models/Lead.php", added: 12, removed: 0 }, { path: "backend/database/migrations/2026_09_05_000001_add_follow_up_to_leads.php", added: 28, removed: 0 }, { path: "backend/app/Jobs/SendFollowUpReminder.php", added: 24, removed: 0 }], note: "Two new columns, one job, no route changes." },
  { type: "metric", title: "Leads followed up this week", value: "38", delta: "+12 vs last week", series: [4, 6, 3, 8, 7, 5, 5] },
]

export const slashCommands: SlashCommand[] = [
  { name: "ship", description: "Drive one unit of work from issue to merged PR through the full gauntlet", source: "project skill", args: "issue <n>" },
  { name: "specops", description: "Query the plans DAG: what to work on next, what is blocked", source: "project skill", args: "next | dag" },
  { name: "spawn-agent", description: "Hand a task to a fresh agent in its own tab", source: "user skill", args: "<id> <model> <effort> <brief> <cwd>" },
  { name: "lavish", description: "Turn a plan or comparison into an annotatable page", source: "user skill" },
  { name: "lavish-live", description: "Annotate the running app, pins come back as fixes", source: "project skill" },
  { name: "code-review", description: "Review the current diff or a PR for bugs and cleanups", source: "built-in", args: "[pr] [--fix]" },
  { name: "simplify", description: "Reuse, simplification and efficiency pass on the changed code", source: "built-in" },
  { name: "security-review", description: "Security review of the pending changes on this branch", source: "built-in" },
  { name: "audit-spec-drift", description: "Compare specs/ against the implementation, three tables", source: "project skill" },
  { name: "grill-me", description: "A relentless interview to sharpen a plan", source: "user skill" },
  { name: "canvas", description: "Draw the current plan as a diagram beside you", source: "user skill" },
  { name: "deploy-prod", description: "Deploy origin/master to prod, with --check first", source: "project skill", args: "--check | --yes | --rollback" },
  { name: "market-price", description: "Where our asking price sits against Carlist and Mudah", source: "project skill", args: "<stock no>" },
  { name: "zoom", description: "Read small text in an image by cropping and magnifying", source: "user skill" },
  { name: "compact", description: "Compact the conversation to free context", source: "built-in" },
  { name: "clear", description: "Start a fresh conversation in this agent", source: "built-in" },
  { name: "model", description: "Switch the model for this agent", source: "built-in", args: "<id>" },
  { name: "cost", description: "Show tokens and cost for this session", source: "built-in" },
  { name: "frontend-design", description: "Distinctive visual direction for new UI", source: "plugin" },
  { name: "tdd", description: "Build the feature test-first, red green refactor", source: "plugin" },
]

export const feed: FeedEvent[] = [
  { id: "f-1", kind: "user", at: "2026-09-05T00:41:00+08:00", text: "Add the two follow-up fields to leads, with the reminder the morning after. Spec is in specs/behaviors/crm-lead-follow-up.md." },
  { id: "f-2", kind: "thinking", at: "2026-09-05T00:41:04+08:00", text: "Reading the spec and the Lead model before touching anything." },
  { id: "f-3", kind: "tool", at: "2026-09-05T00:41:06+08:00", tool: "Read", input: "specs/behaviors/crm-lead-follow-up.md", output: "# Behavior: lead follow-up\nEvery lead carries a next follow-up date and a one-line note.\nThe morning after a lead comes in, the salesperson gets one reminder.", ms: 12, ok: true },
  { id: "f-4", kind: "tool", at: "2026-09-05T00:41:07+08:00", tool: "Grep", input: "next_follow_up_at in backend/", output: "0 matches", ms: 340, ok: true },
  { id: "f-5", kind: "assistant", at: "2026-09-05T00:41:20+08:00", text: "Nothing exists yet. I will add a migration, extend the Lead model, and create the reminder job. Three files." },
  { id: "f-6", kind: "diff", at: "2026-09-05T00:42:10+08:00", file: "backend/app/Models/Lead.php", added: 12, removed: 0, hunk: "@@ -8,6 +8,18 @@\n     protected $fillable = [\n         'customer_id',\n         'vehicle_id',\n         'source',\n+        'next_follow_up_at',\n+        'follow_up_note',\n     ];\n+\n+    protected function casts(): array\n+    {\n+        return [\n+            'next_follow_up_at' => 'datetime',\n+        ];\n+    }" },
  { id: "f-7", kind: "diff", at: "2026-09-05T00:42:40+08:00", file: "backend/database/migrations/2026_09_05_000001_add_follow_up_to_leads.php", added: 28, removed: 0, hunk: "@@ -0,0 +1,28 @@\n+<?php\n+\n+use Illuminate\\Database\\Migrations\\Migration;\n+use Illuminate\\Database\\Schema\\Blueprint;\n+use Illuminate\\Support\\Facades\\Schema;\n+\n+return new class extends Migration\n+{\n+    public function up(): void\n+    {\n+        Schema::table('leads', function (Blueprint $table) {\n+            $table->timestamp('next_follow_up_at')->nullable();\n+            $table->string('follow_up_note')->nullable();\n+        });\n+    }\n+};" },
  { id: "f-8", kind: "subagent", at: "2026-09-05T00:44:00+08:00", name: "Explore", summary: "Found how the PWA lead card reads its fields: LeadCard.tsx, LeadResource.php", events: 9 },
  { id: "f-9", kind: "card", at: "2026-09-05T00:45:30+08:00", card: cards[1] },
  { id: "f-10", kind: "assistant", at: "2026-09-05T00:45:40+08:00", text: "Three leads have no follow-up date today. After the migration I can set them to tomorrow 08:00 if you want." },
  { id: "f-10b", kind: "command", at: "2026-09-05T01:09:20+08:00", name: "tdd", args: "the reminder job, red first", source: "plugin", description: "Build the feature test-first, red green refactor" },
  { id: "f-10c", kind: "assistant", at: "2026-09-05T01:09:30+08:00", text: "Writing the failing test first, then the job." },
  { id: "f-11", kind: "tool", at: "2026-09-05T01:09:50+08:00", tool: "Bash", input: "docker compose exec -T app php vendor/bin/pest tests/Feature/FollowUpReminderTest.php", output: "PASS  Tests\\Feature\\FollowUpReminderTest\n✓ sends one reminder the morning after\n✓ skips leads with no date\n\nTests: 2 passed (4 assertions)\nDuration: 1.92s", ms: 2410, ok: true },
  { id: "f-12", kind: "permission", at: "2026-09-05T01:10:12+08:00", tool: "Bash", command: "docker compose exec -T app php artisan migrate --force" },
]

export const result = {
  agentId: "heron",
  workspaceId: "project-jag",
  title: "Tile jumps when a photo finishes uploading",
  summary: "The upload tile was marked as uploaded before the snapshot was committed, so the grid re-sorted under your finger. Now the row waits for the committed snapshot. The tile stays where it is, and the photo fades in.",
  pr: { number: 648, title: "fix(media): keep the upload tile in place until the snapshot commits", url: "https://github.com/screamyx/project-jag/pull/648", ci: "green", reviews: 1 },
  files: [
    { path: "backend/staff-react/src/media/UploadTile.tsx", added: 22, removed: 9 },
    { path: "backend/staff-react/src/media/useUploadQueue.ts", added: 14, removed: 3 },
    { path: "backend/staff-react/src/media/UploadTile.test.tsx", added: 5, removed: 0 },
  ],
  checks: [
    { name: "backend tests", state: "passed" },
    { name: "staff-react vitest", state: "passed" },
    { name: "browser check on phone", state: "passed" },
  ],
  steps: [
    { name: "PR opened", state: "done", at: "2026-09-05T00:31:00+08:00" },
    { name: "CI green", state: "done", at: "2026-09-05T00:36:00+08:00" },
    { name: "Merge to v2", state: "ready" },
    { name: "Deploy to prod", state: "later", note: "v2 ships on 2026-11-01" },
  ],
}

export const settings = {
  user: { name: "Azani", email: "muhd.azani@gmail.com", auth: "claude.ai OAuth, Max plan" },
  daemon: { host: "pc-ajim.tail82fec1.ts.net", version: "0.1.0", claude: "2.1.260", uptime: "3d 4h" },
  notifications: { push: true, needsYou: true, results: true, quietFrom: "23:30", quietTo: "07:30" },
  models: ["claude-fable-5-1", "claude-opus-5", "claude-sonnet-5", "gpt-5.6-sol"],
}

export const fmtTime = (iso: string) => new Date(iso).toLocaleTimeString("en-MY", { hour: "2-digit", minute: "2-digit", hour12: false })
export const agentById = (id: string) => agents.find((a) => a.id === id)!
export const workspaceById = (id: string) => workspaces.find((w) => w.id === id)!
