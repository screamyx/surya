// Slash commands are whatever Claude Code reports at session start. Nothing here is surya's own.
export type SlashSource = "built-in" | "user skill" | "project skill" | "plugin"
export type SlashCommand = { name: string; description: string; source: SlashSource; args?: string }

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
