// The pin overlay and the pin list, shared by both preview engines.
//
// Decision 21: "Pins still resolve to a selector: the daemon asks Chrome which element is
// under the click, and stores selector plus crop as before." So a pin on a third-party page
// is the same object as a pin on the app, and it wears the same bubble, the same number and
// the same row. One component, both engines.
import { Link } from "react-router"
import { motion } from "motion/react"
import { Send } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { Item, ItemContent, ItemDescription, ItemGroup, ItemMedia, ItemTitle } from "@/components/ui/item"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { tasks, type Pin } from "@/data"
import { markerTone, statusLabel, statusVariant } from "@/screens/session/preview/pin-utils"
import { pop, rise, stagger } from "@/motion"
import { cn } from "@/lib/utils"

const taskTitle = (id?: string) => tasks.find((t) => t.id === id)?.title

function PinBody({ p, n, onSend, onNote }: {
  p: Pin; n: number; onSend: () => void; onNote?: (text: string) => void
}) {
  const linked = taskTitle(p.taskId)
  return (
    <>
      {p.note ? (
        <p className="leading-snug">{p.note}</p>
      ) : (
        <Textarea
          autoFocus
          value={p.note}
          onChange={(e) => onNote?.(e.target.value)}
          placeholder="Say what the agent should do here"
          aria-label={`Note for pin ${n}`}
          className="min-h-14 text-sm"
        />
      )}
      <p className="text-muted-foreground font-mono text-xs break-all">{p.selector}</p>
      <div className="flex flex-wrap items-center gap-1.5">
        <Badge variant="outline">{p.by === "you" ? "You" : "Agent"}</Badge>
        <Badge variant={statusVariant[p.status]}>{statusLabel[p.status]}</Badge>
      </div>
      {linked && <p className="text-muted-foreground text-xs">Task: {linked}</p>}
      <div className="flex flex-wrap gap-2">
        <Button size="sm" onClick={onSend} disabled={!p.note}>
          <Send data-icon="inline-start" />Send to the agent
        </Button>
        <Button
          size="sm"
          variant="outline"
          nativeButton={false}
          render={<Link to={`/w/${p.workspaceId}/tasks`} />}
        >
          {p.taskId ? "Open task" : "Make it a task"}
        </Button>
      </div>
    </>
  )
}

export function Marker({ p, n, selected, fresh, onSelect, onSend, onNote }: {
  p: Pin
  n: number
  selected: boolean
  // A pin you just dropped opens on arrival, because it has no note yet.
  fresh?: boolean
  onSelect: () => void
  onSend: () => void
  onNote?: (text: string) => void
}) {
  return (
    <motion.div
      variants={pop}
      initial="hidden"
      animate="show"
      data-pin-marker
      className="absolute z-10 -translate-x-1/2 -translate-y-1/2"
      style={{ left: `${p.x}%`, top: `${p.y}%` }}
    >
      <Popover defaultOpen={fresh}>
        <PopoverTrigger
          onClick={onSelect}
          className={cn(
            "flex size-7 items-center justify-center rounded-full text-xs font-semibold shadow-lift ring-2 ring-background transition-transform hover:scale-110",
            markerTone(p),
            selected && "ring-ring scale-110",
          )}
          aria-label={`Pin ${n}: ${p.note || "no note yet"}`}
        >
          {n}
        </PopoverTrigger>
        <PopoverContent className="w-72 gap-2">
          <PinBody p={p} n={n} onSend={onSend} onNote={onNote} />
        </PopoverContent>
      </Popover>
    </motion.div>
  )
}

export function PinList({ list, selected, onSelect, onSend }: {
  list: Pin[]; selected: string; onSelect: (id: string) => void; onSend: (p: Pin, n: number) => void
}) {
  return (
    <motion.div variants={stagger} initial="hidden" animate="show">
      <ItemGroup className="gap-2">
        {list.map((p, i) => {
          const linked = taskTitle(p.taskId)
          const active = selected === p.id
          return (
            <motion.div key={p.id} variants={rise}>
              <Item
                variant="outline"
                size="sm"
                role="button"
                tabIndex={0}
                aria-pressed={active}
                onClick={() => onSelect(p.id)}
                onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect(p.id) } }}
                className={cn("bg-card cursor-pointer items-start", active && "border-primary")}
              >
                <ItemMedia>
                  <span className={cn("flex size-6 items-center justify-center rounded-full text-xs font-semibold", markerTone(p))}>
                    {i + 1}
                  </span>
                </ItemMedia>
                <ItemContent className="gap-1.5">
                  <ItemTitle className={cn("line-clamp-2 whitespace-normal", !p.note && "text-muted-foreground italic")}>
                    {p.note || "No note yet"}
                  </ItemTitle>
                  <ItemDescription className="text-muted-foreground font-mono text-xs break-all">{p.selector}</ItemDescription>
                  <div className="flex flex-wrap items-center gap-1.5">
                    <Badge variant="outline">{p.by === "you" ? "You" : "Agent"}</Badge>
                    <Badge variant={statusVariant[p.status]}>{statusLabel[p.status]}</Badge>
                  </div>
                  {linked && <p className="text-muted-foreground truncate text-xs">Task: {linked}</p>}
                </ItemContent>
                {active && p.note && (
                  <Button size="xs" variant="outline" className="shrink-0" onClick={() => onSend(p, i + 1)}>
                    <Send data-icon="inline-start" />Send
                  </Button>
                )}
              </Item>
            </motion.div>
          )
        })}
      </ItemGroup>
    </motion.div>
  )
}
