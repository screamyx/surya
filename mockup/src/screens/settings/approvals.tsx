// Settings panel: the approval policy. Decision 20 point 3 - a permission answer becomes
// a rule, and the rules live here. Two parts, and the split is deliberate:
// the five capability defaults are Cline's model (docs/research/cline.md), the specific
// rules with a pattern and a scope are Kiro's (docs/research/kiro.md).
import { useState } from "react"
import { AnimatePresence, motion } from "motion/react"
import { Check, Eye, Globe, Pencil, Plug, ShieldCheck, Terminal } from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Separator } from "@/components/ui/separator"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { capabilityPolicy, type Capability, type PolicyMode } from "@/settings-data"
import { RulesList } from "@/screens/settings/approval-rules"
import { BeatCount } from "@/screens/beat"
import { pop } from "@/motion"

const capIcon: Record<Capability, typeof Eye> = {
  read: Eye,
  edit: Pencil,
  command: Terminal,
  browser: Globe,
  mcp: Plug,
}

// The check that marks the chosen mode. It carries data-icon itself so the button keeps
// its icon spacing whether the mark is there or not.
function Mark({ on }: { on: boolean }) {
  return (
    <AnimatePresence initial={false}>
      {on && (
        <motion.span key="mark" data-icon="inline-start" variants={pop} initial="hidden" animate="show" exit="hidden" className="inline-flex">
          <Check />
        </motion.span>
      )}
    </AnimatePresence>
  )
}

function CapabilityRow({ capability, label, description, mode, onChange }: {
  capability: Capability
  label: string
  description: string
  mode: PolicyMode
  onChange: (m: PolicyMode) => void
}) {
  const Icon = capIcon[capability]
  return (
    <div className="flex flex-col gap-2.5 py-2.5 sm:flex-row sm:items-center sm:gap-4">
      <Icon className="text-muted-foreground size-4 shrink-0 max-sm:hidden" />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium">{label}</p>
        <p className="text-muted-foreground text-sm">{description}</p>
      </div>
      <ToggleGroup
        value={[mode]}
        onValueChange={(v) => { const next = (v as string[])[0]; if (next) onChange(next as PolicyMode) }}
        variant="outline"
        spacing={0}
        size="sm"
        className="shrink-0 max-sm:w-full"
        aria-label={`${label}: ask or allow`}
      >
        {/* The check keeps the chosen mode readable without leaning on the fill alone.
            It pops in and out so the flip between the two modes is legible as a move. */}
        <ToggleGroupItem value="ask" className="max-sm:flex-1 sm:min-w-24">
          <Mark on={mode === "ask"} />
          Ask first
        </ToggleGroupItem>
        <ToggleGroupItem value="allow" className="max-sm:flex-1 sm:min-w-24">
          <Mark on={mode === "allow"} />
          Allow
        </ToggleGroupItem>
      </ToggleGroup>
    </div>
  )
}

export function ApprovalsPanel() {
  const [modes, setModes] = useState<Record<string, PolicyMode>>(
    Object.fromEntries(capabilityPolicy.map((c) => [c.capability, c.mode]))
  )
  const asking = capabilityPolicy.filter((c) => modes[c.capability] === "ask").length

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ShieldCheck className="text-muted-foreground size-4" />
          Approvals
        </CardTitle>
        <CardDescription>
          What an agent may do on its own, and the answers you have already kept. Every rule here started
          as an ask in your inbox.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <p className="text-sm">
          <BeatCount value={asking} className="font-medium" /> of{" "}
          <span className="tnum font-medium">{capabilityPolicy.length}</span> capabilities ask you first.
        </p>
        <div className="mt-2 flex flex-col divide-y">
          {capabilityPolicy.map((c) => (
            <CapabilityRow
              key={c.capability}
              capability={c.capability}
              label={c.label}
              description={c.description}
              mode={modes[c.capability]}
              onChange={(m) => setModes((prev) => ({ ...prev, [c.capability]: m }))}
            />
          ))}
        </div>

        <Separator className="my-4" />
        <RulesList />
      </CardContent>
    </Card>
  )
}
