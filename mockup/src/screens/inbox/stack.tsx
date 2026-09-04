// Stacked correspondence, not a grid. The first item waiting on you is open at full
// weight; everything under it is one condensed line until you open it. One is open at
// a time, so the eye always has a single place to land.
//
// Answering a card is the one thing that changes this list, so it is what moves: the
// card collapses to a line, drops below the ones still waiting, and the next waiting
// item opens in the space it left. Every step is on the same layout spring, so the
// stack settles rather than redrawing.
import { useState } from "react"
import { AnimatePresence, motion } from "motion/react"
import { ChevronRight, MapPin, MessageCircleQuestion, OctagonX, PackageCheck, ShieldAlert } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { agentById, fmtTime, workspaceById, type InboxItem, type InboxKind } from "@/data"
import { layoutSpring, pop, rise, stagger, swap } from "@/motion"
import { InboxCard } from "@/screens/inbox/cards"
import { cn } from "@/lib/utils"

const mark: Record<InboxKind, { icon: typeof ShieldAlert; word: string; tone: string }> = {
  permission: { icon: ShieldAlert, word: "Permission", tone: "text-foreground" },
  question: { icon: MessageCircleQuestion, word: "Question", tone: "text-foreground" },
  failed: { icon: OctagonX, word: "Stopped", tone: "text-destructive" },
  pin: { icon: MapPin, word: "Pin", tone: "text-muted-foreground" },
  result: { icon: PackageCheck, word: "Result", tone: "text-ok" },
}

export const waiting = (i: InboxItem) => i.kind === "permission" || i.kind === "question" || i.kind === "failed"

// Waiting items come first, so the item that leads the screen is one that needs an answer.
export const byUrgency = (list: InboxItem[]) =>
  [...list].sort((a, b) => Number(waiting(b)) - Number(waiting(a)))

function Line({ item, tail, onClick, muted }: {
  item: InboxItem
  tail: React.ReactNode
  onClick?: () => void
  muted?: boolean
}) {
  const m = mark[item.kind]
  const Icon = m.icon
  const agent = agentById(item.agentId)
  const ws = workspaceById(item.workspaceId)
  const body = (
    <>
      <span className={cn("bg-muted flex size-8 shrink-0 items-center justify-center rounded-md", muted ? "text-muted-foreground" : m.tone)}>
        <Icon className="size-4" />
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="flex items-center gap-2">
          <span className={cn("truncate text-sm font-medium", muted && "text-muted-foreground")}>{item.title}</span>
          {!muted && waiting(item) && <Badge variant="outline" className="hidden shrink-0 sm:inline-flex">Waiting</Badge>}
        </span>
        <span className="text-muted-foreground truncate text-xs">
          {m.word} · {agent.name} · <span className="font-mono">{ws.name}</span>
        </span>
      </span>
      {tail}
    </>
  )
  if (!onClick) {
    return <div className="flex w-full items-center gap-3 rounded-lg border border-transparent px-3 py-2.5">{body}</div>
  }
  return (
    <button
      type="button"
      onClick={onClick}
      className="hover:bg-card focus-visible:ring-ring/50 focus-visible:border-ring flex w-full items-center gap-3 rounded-lg border border-transparent px-3 py-2.5 text-left transition-colors outline-none focus-visible:ring-3"
    >
      {body}
    </button>
  )
}

// A line that has been answered. It says what the answer was and offers the way back,
// so an item that leaves the open slot never leaves without an account of itself.
function SettledRow({ item, outcome, onUndo }: { item: InboxItem; outcome: string; onUndo: () => void }) {
  return (
    <Line
      item={item}
      muted
      tail={
        <>
          <Badge variant="secondary" className="shrink-0">{outcome}</Badge>
          <Button variant="ghost" size="sm" className="text-muted-foreground shrink-0" onClick={onUndo}>Undo</Button>
        </>
      }
    />
  )
}

function CondensedRow({ item, onOpen }: { item: InboxItem; onOpen: () => void }) {
  return (
    <Line
      item={item}
      onClick={onOpen}
      tail={
        <>
          <span className="text-muted-foreground tnum shrink-0 text-xs">{fmtTime(item.at)}</span>
          <ChevronRight className="text-muted-foreground size-4 shrink-0" />
        </>
      }
    />
  )
}

// Insertion order matters: an answered item drops to the bottom in the order it was
// answered, so the list reads as a history rather than reshuffling itself.
export type Settled = { id: string; outcome: string }

export function InboxStack({ items, settled, onSettle }: {
  items: InboxItem[]
  settled: Settled[]
  onSettle: (next: Settled[]) => void
}) {
  const ordered = byUrgency(items)
  const [openId, setOpenId] = useState(ordered[0]?.id)
  const setSettled = (fn: (prev: Settled[]) => Settled[]) => onSettle(fn(settled))

  const outcomeOf = (id: string) => settled.find((s) => s.id === id)?.outcome
  const live = ordered.filter((i) => !outcomeOf(i.id))
  const closed = settled
    .map((s) => ({ item: ordered.find((i) => i.id === s.id), outcome: s.outcome }))
    .filter((s): s is { item: InboxItem; outcome: string } => Boolean(s.item))

  const resolve = (id: string, outcome: string) => {
    setSettled((prev) => (prev.some((s) => s.id === id) ? prev : [...prev, { id, outcome }]))
    // The next item still waiting takes the open slot the answered one gave up.
    const rest = live.filter((i) => i.id !== id)
    setOpenId((rest.find(waiting) ?? rest[0])?.id)
  }

  const undo = (id: string) => {
    setSettled((prev) => prev.filter((s) => s.id !== id))
    setOpenId(id)
  }

  return (
    <motion.div variants={stagger} initial="hidden" animate="show" className="flex flex-col gap-1.5">
      <AnimatePresence initial={false}>
        {live.map((item) => (
          <motion.div key={item.id} layout="position" transition={layoutSpring} variants={rise}>
            <AnimatePresence mode="wait" initial={false}>
              {item.id === openId ? (
                <motion.div key="card" variants={pop} initial="hidden" animate="show" exit="hidden">
                  <InboxCard item={item} onResolve={(outcome) => resolve(item.id, outcome)} />
                </motion.div>
              ) : (
                <motion.div key="row" variants={swap} initial="hidden" animate="show" exit="exit">
                  <CondensedRow item={item} onOpen={() => setOpenId(item.id)} />
                </motion.div>
              )}
            </AnimatePresence>
          </motion.div>
        ))}

        {closed.map(({ item, outcome }) => (
          <motion.div key={item.id} layout="position" transition={layoutSpring} variants={swap} initial="hidden" animate="show" exit="exit">
            <SettledRow item={item} outcome={outcome} onUndo={() => undo(item.id)} />
          </motion.div>
        ))}
      </AnimatePresence>
    </motion.div>
  )
}
