export const settings = {
  user: { name: "Alex Tan", email: "alex@example.com", auth: "claude.ai OAuth, Max plan" },
  daemon: { host: "studio.local", version: "0.1.0", claude: "2.1.260", uptime: "3d 4h" },
  notifications: { push: true, needsYou: true, results: true, quietFrom: "23:30", quietTo: "07:30" },
  models: ["claude-fable-5-1", "claude-opus-5", "claude-sonnet-5", "gpt-5.6-sol"],
}

// What Claude Code's own config files say. surya reads these, it does not own them.
export type McpServer = { name: string; transport: "stdio" | "http"; command: string; state: "connected" | "not connected" | "disabled"; tools: number; scope: "user" | "project" | "local" }
export const mcpServers: McpServer[] = [
  { name: "laravel-boost", transport: "stdio", command: "docker compose exec -T app php artisan boost:mcp", state: "connected", tools: 11, scope: "project" },
  { name: "agb", transport: "stdio", command: "/usr/local/bin/agb mcp", state: "connected", tools: 10, scope: "user" },
  { name: "context7", transport: "http", command: "https://mcp.context7.com/mcp", state: "connected", tools: 2, scope: "user" },
  { name: "code-search", transport: "http", command: "http://127.0.0.1:8377/mcp", state: "connected", tools: 2, scope: "user" },
  { name: "open-claude-in-chrome", transport: "stdio", command: "claude-in-chrome-mcp", state: "connected", tools: 20, scope: "user" },
  { name: "windows-dtry", transport: "stdio", command: "ssh dtry windows-mcp", state: "connected", tools: 10, scope: "user" },
  { name: "fb_ad_library", transport: "stdio", command: "tools/facebook-ads-library-mcp/run.sh", state: "not connected", tools: 0, scope: "project" },
]

export type Plugin = { name: string; version: string; source: string; enabled: boolean; provides: string[] }
export const plugins: Plugin[] = [
  { name: "mattpocock-skills", version: "1.4.0", source: "github.com/mattpocock/skills", enabled: true, provides: ["tdd", "code-review", "grilling", "wizard", "research", "domain-modeling"] },
  { name: "frontend-design", version: "0.9.2", source: "anthropics/claude-plugins", enabled: true, provides: ["frontend-design"] },
  { name: "dataviz", version: "0.3.1", source: "anthropics/claude-plugins", enabled: false, provides: ["dataviz"] },
]

export type SkillEntry = { name: string; description: string; source: "user" | "project" | "plugin"; path: string; enabled: boolean }
export const skills: SkillEntry[] = [
  { name: "ship", description: "Drive one unit of work from issue to merged PR", source: "project", path: ".claude/skills/ship", enabled: true },
  { name: "specops", description: "Query the plans DAG", source: "project", path: ".claude/skills/specops", enabled: true },
  { name: "lavish-live", description: "Annotate the running app", source: "project", path: ".claude/skills/lavish-live", enabled: true },
  { name: "deploy-prod", description: "Deploy origin/master to prod", source: "project", path: ".claude/skills/deploy-prod", enabled: true },
  { name: "spawn-agent", description: "Hand a task to a fresh agent in its own tab", source: "user", path: "~/.claude/skills/spawn-agent", enabled: true },
  { name: "lavish", description: "Turn a plan into an annotatable page", source: "user", path: "~/.claude/skills/lavish", enabled: true },
  { name: "zoom", description: "Read small text in an image", source: "user", path: "~/.claude/skills/zoom", enabled: true },
  { name: "grill-me", description: "A relentless interview to sharpen a plan", source: "user", path: "~/.claude/skills/grill-me", enabled: false },
  { name: "tdd", description: "Build the feature test-first", source: "plugin", path: "mattpocock-skills/tdd", enabled: true },
]

// Approval policy (decision 20, point 3). One answer in the inbox can become a rule,
// and the rules live here. Cline's per-capability model and Kiro's pattern-and-scope
// rules are the references; see docs/research/cline.md and docs/research/kiro.md.
export type Capability = "read" | "edit" | "command" | "browser" | "mcp"
export type PolicyMode = "ask" | "allow"

export const capabilityPolicy: { capability: Capability; label: string; description: string; mode: PolicyMode }[] = [
  { capability: "read", label: "Read files", description: "Open any file in the worktree.", mode: "allow" },
  { capability: "edit", label: "Edit files", description: "Write to files the agent already read.", mode: "allow" },
  { capability: "command", label: "Run commands", description: "Anything in a shell, including migrations and deploys.", mode: "ask" },
  { capability: "browser", label: "Drive the browser", description: "Click through the preview and read the console.", mode: "ask" },
  { capability: "mcp", label: "Call MCP tools", description: "Every tool the connected MCP servers expose.", mode: "ask" },
]

export type ApprovalRule = {
  id: string
  capability: Capability
  label: string       // what you see: "Run database migrations"
  pattern: string     // what it matches: "php artisan migrate*"
  scope: string       // "project-jag", "studio", or "everywhere"
  from: string        // the ask that created it
  createdAt: string
  uses: number
}

export const approvalRules: ApprovalRule[] = [
  { id: "r-1", capability: "command", label: "Run the test suite", pattern: "vendor/bin/pest*", scope: "project-jag", from: "raven asked on 2026-08-29", createdAt: "2026-08-29T14:20:00+08:00", uses: 38 },
  { id: "r-2", capability: "command", label: "Read the git log", pattern: "git log*", scope: "everywhere", from: "kite asked on 2026-08-22", createdAt: "2026-08-22T09:05:00+08:00", uses: 112 },
  { id: "r-3", capability: "mcp", label: "Search the Laravel docs", pattern: "laravel-boost/search-docs", scope: "project-jag", from: "raven asked on 2026-09-01", createdAt: "2026-09-01T22:40:00+08:00", uses: 17 },
  { id: "r-4", capability: "browser", label: "Open the preview", pattern: "localhost:8000/*", scope: "project-jag", from: "heron asked on 2026-09-03", createdAt: "2026-09-03T11:12:00+08:00", uses: 6 },
]
