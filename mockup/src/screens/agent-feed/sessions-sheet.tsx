// Every conversation this agent has had. The daemon reattaches to any of them with
// `claude --resume <id>`, probed 2026-09-05, docs/probes/headless-controls-2026-09-05.md.
import { motion } from "motion/react"
import { Plus } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Item, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item"
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from "@/components/ui/sheet"
import { useIsMobile } from "@/hooks/use-mobile"
import { rise, stagger } from "@/motion"
import { fmtTime, type Session } from "@/data"
import { cn } from "@/lib/utils"
import { modelName } from "@/screens/agent-feed/model-sheet"

// Sessions run across midnight, so the day has to show or "22:10" reads as tonight.
const day = (iso: string) => new Date(iso).toLocaleDateString("en-MY", { day: "numeric", month: "short" })
const when = (iso: string) => `${day(iso)}, ${fmtTime(iso)}`

export function SessionsSheet({ open, onOpenChange, name, sessions }: {
  open: boolean; onOpenChange: (v: boolean) => void; name: string; sessions: Session[]
}) {
  const isMobile = useIsMobile()
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side={isMobile ? "bottom" : "right"} className={cn("gap-0 p-0", isMobile && "max-h-[85svh]")}>
        <SheetHeader className="border-b">
          <SheetTitle>Sessions with {name}</SheetTitle>
          <SheetDescription>{sessions.length} conversations. Pick one up where it stopped.</SheetDescription>
        </SheetHeader>

        <div className="min-h-0 flex-1 overflow-y-auto p-4">
          <Button variant="outline" className="mb-3 w-full">
            <Plus data-icon="inline-start" />New session
          </Button>
          <motion.div key={open ? "open" : "shut"} variants={stagger} initial="hidden" animate="show" className="grid gap-2">
            {sessions.map((s, i) => (
              <motion.div key={s.id} variants={rise}>
                <SessionRow s={s} current={i === 0} />
              </motion.div>
            ))}
          </motion.div>
        </div>

        <SheetFooter className="border-t">
          <p className="text-muted-foreground text-xs">
            surya resumes with <code className="font-mono">claude --resume &lt;id&gt;</code>. Nothing is lost when the daemon restarts.
          </p>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  )
}

function SessionRow({ s, current }: { s: Session; current: boolean }) {
  return (
    <Item variant="outline" className={cn("items-start gap-3", current && "border-primary")}>
      <ItemContent className="gap-1.5">
        <ItemTitle className="line-clamp-2 leading-snug">{s.title}</ItemTitle>
        <ItemDescription className="flex flex-wrap items-center gap-x-2 gap-y-1.5">
          <span className="tabular-nums">{when(s.startedAt)}</span>
          {s.endedAt ? (
            <span className="tabular-nums">ended {fmtTime(s.endedAt)}</span>
          ) : (
            <Badge variant="default"><span className="bg-primary-foreground size-1.5 animate-pulse rounded-full" />live now</Badge>
          )}
          <Badge variant="outline" className="tabular-nums">{s.turns} turns</Badge>
          <Badge variant="outline" className="font-mono">{modelName(s.model)}</Badge>
          {current && <Badge variant="secondary">current</Badge>}
        </ItemDescription>
      </ItemContent>
      <Button size="sm" variant="ghost" className="shrink-0">{current ? "Open" : "Resume"}</Button>
    </Item>
  )
}
