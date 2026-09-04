// The rule a permission ask would become, derived from the ask itself.
// Kiro's approval flow is the model: "Always allow" opens a pattern and scope picker,
// and "the pattern dropdown suggests a generalized version of the specific operation"
// (docs/research/kiro.md, docs/research/shots/kiro-approval-flow.png). The capability
// split is Cline's: read, edit, command, browser, mcp (docs/research/cline.md).
import { serverById, workspaceById, type InboxItem } from "@/data"
import { capabilityPolicy, type Capability } from "@/settings-data"

export type PatternChoice = { id: "broad" | "exact"; pattern: string; hint: string }
export type ScopeChoice = { id: "workspace" | "server" | "everywhere"; label: string; scope: string; hint: string }
export type CreatedRule = { capabilityLabel: string; pattern: string; scope: string }

export type RuleDraft = {
  capability: Capability
  capabilityLabel: string
  noun: string // what the rule covers, in one plain word: "migrations"
  patterns: PatternChoice[]
  scopes: ScopeChoice[]
}

// Which capability a tool belongs to. An MCP tool is written server/tool, so it carries a slash.
function capabilityOf(tool: string): Capability {
  if (tool.includes("/")) return "mcp"
  if (/^(read|grep|glob|ls)$/i.test(tool)) return "read"
  if (/^(edit|write|notebookedit)$/i.test(tool)) return "edit"
  if (/browser|chrome|playwright/i.test(tool)) return "browser"
  return "command"
}

// A command runs through a wrapper more often than not. The rule should match the work,
// not the wrapper, so drop the wrapper before generalising.
function unwrap(command: string): string {
  return command
    .replace(/^docker\s+compose\s+exec\s+(-\S+\s+)*\S+\s+/, "")
    .replace(/^(sudo|env|nice|time)\s+/, "")
    .trim()
}

// The named shapes we can describe in plain words. Anything else falls back to the
// first two words of the command, which is still a rule a person can read.
const shapes: { match: RegExp; noun: string; stem: string }[] = [
  { match: /artisan\s+migrate/, noun: "migrations", stem: "php artisan migrate" },
  { match: /(pest|phpunit|vitest|jest)\b/, noun: "test runs", stem: "vendor/bin/pest" },
  { match: /^git\s+(log|status|diff|show)/, noun: "git reads", stem: "git log" },
  { match: /^npm\s+(run|test)|^bun\s+run/, noun: "build scripts", stem: "bun run" },
]

function generalise(command: string): { noun: string; broad: string } {
  const bare = unwrap(command)
  const named = shapes.find((s) => s.match.test(bare))
  if (named) return { noun: named.noun, broad: `${named.stem}*` }
  const words = bare.split(/\s+/).filter((w) => !w.startsWith("-")).slice(0, 2)
  return { noun: `${words.join(" ")} commands`, broad: `${words.join(" ")}*` }
}

export function ruleDraft(item: InboxItem): RuleDraft {
  const capability = capabilityOf(item.tool ?? "Bash")
  const ws = workspaceById(item.workspaceId)
  const server = serverById(ws.serverId)
  const command = item.command ?? item.tool ?? ""
  const { noun, broad } = generalise(command)

  return {
    capability,
    capabilityLabel: capabilityPolicy.find((c) => c.capability === capability)!.label,
    noun,
    patterns: [
      { id: "broad", pattern: broad, hint: `Every ${noun.replace(/s$/, "")} that matches` },
      { id: "exact", pattern: unwrap(command), hint: "This command and nothing else" },
    ],
    scopes: [
      { id: "workspace", label: "This workspace", scope: ws.name, hint: `Only ${ws.name}` },
      { id: "server", label: "This server", scope: server.name, hint: `Every workspace on ${server.name}` },
      { id: "everywhere", label: "Everywhere", scope: "everywhere", hint: "Every workspace on every server" },
    ],
  }
}
