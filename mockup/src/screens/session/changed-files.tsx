// The review reading of the files pane: what this agent changed, and the diff for the file
// you pick. This is the whole files pane on a phone, because the editor and the tree are
// desktop only (decision 20, point 4) and Cursor's mobile app makes the same cut.
import { useEffect, useRef, useState } from "react"
import { Link } from "react-router"
import { motion } from "motion/react"
import { FileDiff, FilePlus } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Item, ItemContent, ItemMedia } from "@/components/ui/item"
import { rise, stagger } from "@/motion"
import { feed, fmtTime, type Workspace } from "@/data"
import { cn } from "@/lib/utils"
import { ScrollArea } from "@/components/ui/scroll-area"

type Change = { id: string; path: string; added: number; removed: number; hunk: string; at: string }

// What the agent actually wrote in this run, read off its own transcript.
const changes: Change[] = feed
  .filter((e) => e.kind === "diff")
  .map((e) => (e.kind === "diff" ? { id: e.id, path: e.file, added: e.added, removed: e.removed, hunk: e.hunk, at: e.at } : null))
  .filter((c): c is Change => c !== null)

const nameOf = (path: string) => path.split("/").pop() ?? path
const dirOf = (path: string) => path.slice(0, path.length - nameOf(path).length).replace(/\/$/, "")

// A diff line never wraps, so the region scrolls sideways. This says whether it actually
// does, because rule 7 wants a cue a sighted person can see, not only an aria-label.
function useScrollsSideways(dep: unknown) {
  const ref = useRef<HTMLDivElement>(null)
  const [over, setOver] = useState(false)
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const check = () => setOver(el.scrollWidth > el.clientWidth + 1)
    check()
    const ro = new ResizeObserver(check)
    ro.observe(el)
    return () => ro.disconnect()
  }, [dep])
  return { ref, over }
}

function HunkLine({ line }: { line: string }) {
  return (
    <div
      className={cn(
        "px-4 whitespace-pre",
        line.startsWith("@@") && "text-muted-foreground bg-muted/50",
        line.startsWith("+") && "bg-accent text-accent-foreground font-medium",
        line.startsWith("-") && "bg-muted text-muted-foreground line-through",
        !line.startsWith("@@") && !line.startsWith("+") && !line.startsWith("-") && "text-muted-foreground",
      )}
    >
      {line || " "}
    </div>
  )
}

function FileRow({ c, active, onSelect }: { c: Change; active: boolean; onSelect: () => void }) {
  // A new file has nothing removed. Saying so gives the eye something that varies down the list.
  const created = c.removed === 0 && c.added > 20
  return (
    <motion.div variants={rise}>
      <Item
        variant="outline"
        size="sm"
        role="button"
        tabIndex={0}
        aria-pressed={active}
        onClick={onSelect}
        onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect() } }}
        className={cn("hover:bg-muted flex-nowrap cursor-pointer items-center gap-3", active && "border-primary bg-card")}
      >
        <ItemMedia>
          {created
            ? <FilePlus className="text-muted-foreground size-4" />
            : <FileDiff className="text-muted-foreground size-4" />}
        </ItemMedia>
        {/* Plain elements, not ItemTitle: that slot sizes to its content and a long
            migration filename would then push the counts off the pane. */}
        <ItemContent className="min-w-0 flex-1 gap-0.5">
          <span className="truncate font-mono text-sm font-medium">{nameOf(c.path)}</span>
          <span className="text-muted-foreground truncate font-mono text-xs">
            {created ? "new file · " : ""}{dirOf(c.path)}
          </span>
        </ItemContent>
        <span className="flex shrink-0 items-center gap-1.5">
          <Badge variant="secondary" className="tnum">+{c.added}</Badge>
          <Badge variant="outline" className="tnum">-{c.removed}</Badge>
        </span>
      </Item>
    </motion.div>
  )
}

function Hunk({ current, ws, compact }: { current: Change; ws: Workspace; compact: boolean }) {
  const { ref, over } = useScrollsSideways(current.id)
  return (
    <section className={cn("bg-card flex min-h-0 flex-col overflow-hidden rounded-xl border shadow-raise", !compact && "flex-1")}>
      <header className="flex min-w-0 shrink-0 flex-wrap items-center gap-2 border-b px-3 py-2">
        <span className="min-w-0 flex-1 truncate font-mono text-xs">{current.path}</span>
        <span className="text-muted-foreground text-xs tnum">{fmtTime(current.at)}</span>
        {!compact && (
          <Button size="xs" variant="ghost" nativeButton={false} render={<Link to={`/w/${ws.id}/agent/raven/files`} />}>
            Open in editor
          </Button>
        )}
      </header>
      <div className="relative min-h-0 flex-1">
        <div ref={ref} className={cn("overflow-x-auto py-1 font-mono text-xs leading-relaxed", !compact && "h-full overflow-y-auto")}>
          <div className="min-w-max">
            {current.hunk.split("\n").map((line, i) => <HunkLine key={i} line={line} />)}
          </div>
        </div>
        {/* The edge fade appears only when there really is more to the right. */}
        {over && <div className="from-card pointer-events-none absolute inset-y-0 right-0 w-8 bg-linear-to-l to-transparent" />}
      </div>
    </section>
  )
}

export function ChangedFiles({ ws, compact }: { ws: Workspace; compact: boolean }) {
  const [selected, setSelected] = useState(changes[0].id)
  const current = changes.find((c) => c.id === selected) ?? changes[0]
  const added = changes.reduce((n, c) => n + c.added, 0)
  const removed = changes.reduce((n, c) => n + c.removed, 0)

  const head = (
    <>
      <div>
        <h2 className="text-lg font-medium">Changed in this run</h2>
        <p className="text-muted-foreground mt-0.5 text-sm">
          On <span className="font-mono">{ws.branch}</span>, not committed yet.
        </p>
      </div>
      <motion.div variants={stagger} initial="hidden" animate="show" className="grid min-w-0 grid-cols-[minmax(0,1fr)] gap-2">
        {changes.map((c) => (
          <FileRow key={c.id} c={c} active={c.id === selected} onSelect={() => setSelected(c.id)} />
        ))}
      </motion.div>
    </>
  )

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      {compact ? (
        // One scroller on a phone, so the diff never fights the pane for the same gesture.
        <ScrollArea className="min-h-0 min-w-0 flex-1">
          <div className="min-w-0 space-y-4 p-4">
            {head}
            <Hunk current={current} ws={ws} compact />
          </div>
        </ScrollArea>
      ) : (
        <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-4 p-4">
          {head}
          <Hunk current={current} ws={ws} compact={false} />
        </div>
      )}

      {/* The pane ends on a count, so you can see the review is complete. */}
      <footer className="bg-background text-muted-foreground flex shrink-0 items-center gap-3 border-t px-4 py-2.5 text-xs">
        <span className="shrink-0 tnum">{changes.length} files changed</span>
        <span className="text-ok shrink-0 tnum">+{added}</span>
        <span className="shrink-0 tnum">-{removed}</span>
        <span className="ml-auto hidden min-w-0 truncate font-mono sm:block">{ws.worktree}</span>
      </footer>
    </div>
  )
}
