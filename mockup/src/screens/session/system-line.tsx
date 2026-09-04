// Rows surya writes into the feed itself, not ones Claude Code sent: the model switch it confirmed
// and the stop you pressed. Kept local to this screen so data.ts stays the recorded transcript.
import type { LucideIcon } from "lucide-react"
import { fmtTime } from "@/data"

// `at` is left off when the sentence already carries the time, so the stamp never doubles up.
export type SystemRow = { id: string; text: string; at?: string; Icon: LucideIcon }

export function SystemLine({ row }: { row: SystemRow }) {
  const { Icon } = row
  return (
    <div className="flex justify-center">
      <div className="text-muted-foreground bg-muted/50 flex max-w-full items-center gap-2 rounded-2xl border px-3 py-1.5 text-xs">
        <Icon className="size-3.5 shrink-0" />
        <span className="min-w-0">{row.text}</span>
        {row.at && <span className="shrink-0 tabular-nums opacity-70">{fmtTime(row.at)}</span>}
      </div>
    </div>
  )
}
