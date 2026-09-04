// Collapsing the run. Codex folds a stretch of machine work into one line, "Worked for
// 4m 12s", and opens a readable summary on demand (docs/research/codex.md, job 2).
// Cursor writes an automatic checkpoint before a run so you can go back (cursor.md, job 2).
// A permission card and a question never fold: those are the two things that stop the run.
import { useState } from "react"
import { motion } from "motion/react"
import { ChevronRight, History } from "lucide-react"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import { fade, rise } from "@/motion"
import { fmtTime, type FeedEvent } from "@/data"
import { cn } from "@/lib/utils"

// Machine work. Everything else is a person's turn, an artifact, or a stop.
const foldable = new Set<FeedEvent["kind"]>(["tool", "thinking", "subagent", "mail-out"])
export const isFoldable = (e: FeedEvent) => foldable.has(e.kind)

export type Run = { id: string; events: FeedEvent[] }
export type Strand = { kind: "run"; run: Run } | { kind: "event"; event: FeedEvent }

// Two adjacent tool calls are a run. One on its own stays a row, because folding it
// hides more than it saves.
export function strands(events: FeedEvent[]): Strand[] {
  const out: Strand[] = []
  let run: FeedEvent[] = []
  const flush = () => {
    if (run.length > 1) out.push({ kind: "run", run: { id: `run-${run[0].id}`, events: run } })
    else for (const e of run) out.push({ kind: "event", event: e })
    run = []
  }
  for (const e of events) {
    if (isFoldable(e)) run.push(e)
    else { flush(); out.push({ kind: "event", event: e }) }
  }
  flush()
  return out
}

// "4m 12s", the shape Codex uses. Seconds only under a minute.
export function spanLabel(from: string, to: string) {
  const s = Math.max(1, Math.round((Date.parse(to) - Date.parse(from)) / 1000))
  const m = Math.floor(s / 60)
  return m ? `${m}m ${s % 60}s` : `${s}s`
}

// What the run actually did, in plain words, so the closed line still tells you something.
function summarise(events: FeedEvent[]) {
  const n = (k: FeedEvent["kind"]) => events.filter((e) => e.kind === k).length
  const parts: string[] = []
  const tools = n("tool")
  if (tools) parts.push(`${tools} ${tools === 1 ? "tool call" : "tool calls"}`)
  const subs = n("subagent")
  if (subs) parts.push(`${subs} ${subs === 1 ? "subagent" : "subagents"}`)
  const mail = n("mail-out")
  if (mail) parts.push(`${mail} ${mail === 1 ? "message" : "messages"}`)
  const thought = n("thinking")
  if (thought) parts.push("thinking")
  return parts.join(", ")
}

export function RunFold({ run, children }: { run: Run; children: React.ReactNode }) {
  const [open, setOpen] = useState(false)
  const first = run.events[0]
  const last = run.events[run.events.length - 1]
  return (
    <motion.div variants={rise}>
      <Collapsible open={open} onOpenChange={setOpen}>
        <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
          <CollapsibleTrigger className="text-muted-foreground hover:text-foreground flex min-w-0 items-center gap-2 rounded-md py-1 text-sm transition-colors">
            <ChevronRight className={cn("size-3.5 shrink-0 transition-transform", open && "rotate-90")} />
            <span className="text-foreground font-medium">Worked for {spanLabel(first.at, last.at)}</span>
            <span className="min-w-0 truncate text-xs">{summarise(run.events)}</span>
          </CollapsibleTrigger>
          {/* surya writes a checkpoint before every run, so the timeline says where you can go back to. */}
          <span className="text-muted-foreground ml-auto flex shrink-0 items-center gap-1.5 text-xs">
            <History className="size-3.5" />
            checkpoint <span className="tnum">{fmtTime(first.at)}</span>
          </span>
        </div>
        <CollapsibleContent>
          <motion.div variants={fade} initial="hidden" animate="show" className="mt-2 space-y-3 border-l pl-4">
            {children}
          </motion.div>
        </CollapsibleContent>
      </Collapsible>
    </motion.div>
  )
}
