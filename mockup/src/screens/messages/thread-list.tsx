// Left column: the workspace channel, then one row per agent, unread first.
import { motion } from "motion/react"
import { Hash } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { StatusDot } from "@/components/status"
import { fmtTime } from "@/data"
import { rise, stagger } from "@/motion"
import type { Thread } from "@/screens/messages/threads"
import { cn } from "@/lib/utils"

function Preview({ t }: { t: Thread }) {
  if (!t.last) return <span className="text-muted-foreground/70 truncate text-xs">No mail yet</span>
  const who = t.last.from === "you" ? "you" : t.last.from
  return (
    <span className="text-muted-foreground truncate text-xs">
      {who}: {t.last.text}
    </span>
  )
}

export function ThreadList({ threads, selected, onSelect, className }: {
  threads: Thread[]
  selected: string
  onSelect: (id: string) => void
  className?: string
}) {
  return (
    <motion.div variants={stagger} initial="hidden" animate="show" className={cn("flex flex-col gap-1 p-2", className)}>
      {threads.map((t) => (
        <motion.button
          key={t.id}
          type="button"
          variants={rise}
          onClick={() => onSelect(t.id)}
          aria-current={selected === t.id}
          className={cn(
            "hover:bg-muted flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-left transition-colors",
            selected === t.id && "bg-muted",
          )}
        >
          {t.kind === "channel" ? (
            <Hash className="text-muted-foreground size-4 shrink-0" />
          ) : (
            <StatusDot status={t.agent.status} className="ml-1 shrink-0" />
          )}
          <span className="flex min-w-0 flex-1 flex-col gap-0.5">
            <span className="flex items-center gap-2">
              <span className="truncate text-sm font-medium">{t.label}</span>
              {t.last && <span className="text-muted-foreground ml-auto shrink-0 text-xs tabular-nums">{fmtTime(t.last.at)}</span>}
            </span>
            <Preview t={t} />
          </span>
          {t.unread > 0 && <Badge className="shrink-0 tabular-nums">{t.unread}</Badge>}
        </motion.button>
      ))}
    </motion.div>
  )
}
