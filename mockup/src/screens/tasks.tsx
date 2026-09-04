// Screen: /w/:ws/tasks - the task board per workspace.
// Four equal columns, three of them nearly empty, is four equal wells of nothing.
// The columns are weighted by what they hold: Running is the column a person came to
// look at, Blocked is the column that needs a decision, so those two get the room and
// the full card. Queued and Done are narrow lists. Every card surfaces what varies
// about it: how long it has run, how long it has waited, what it is waiting on.
import { useState } from "react"
import { Link, useParams } from "react-router"
import { AnimatePresence, motion } from "motion/react"
import { MapPin, Plus, Sparkles, User } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { StatusDot } from "@/components/status"
import { BeatCount } from "@/screens/beat"
import { agentById, fmtTime, now, tasks, workspaceById, workspaces, type Task, type TaskStatus } from "@/data"
import { layoutSpring, rise, stagger, swap } from "@/motion"
import { cn } from "@/lib/utils"

type Weight = "lead" | "wide" | "narrow"

const columns: { status: TaskStatus; label: string; note: string; empty: string; weight: Weight }[] = [
  {
    status: "queued", label: "Queued", weight: "narrow",
    note: "Next idle agent takes it",
    empty: "Nothing queued. Add a task and the next idle agent picks it up.",
  },
  {
    status: "running", label: "Running", weight: "lead",
    note: "An agent is on it now",
    empty: "No agent is working in this workspace right now.",
  },
  {
    status: "blocked", label: "Blocked", weight: "wide",
    note: "Waiting on a task that has not landed",
    empty: "Nothing is blocked. Every task here can start.",
  },
  {
    status: "done", label: "Done", weight: "narrow",
    note: "Finished, waiting to be shipped",
    empty: "Nothing finished in this workspace yet.",
  },
]

const sourceMeta = {
  you: { label: "You", icon: User, variant: "outline" as const },
  pin: { label: "From a pin", icon: MapPin, variant: "secondary" as const },
  agent: { label: "Agent", icon: Sparkles, variant: "secondary" as const },
}

// How long, against the mockup's fixed "now". This is what varies between two cards
// that otherwise look the same, so it is the thing worth printing.
function elapsed(iso: string) {
  const mins = Math.max(0, Math.round((Date.parse(now) - Date.parse(iso)) / 60000))
  if (mins < 60) return `${mins}m`
  const h = Math.floor(mins / 60)
  return mins % 60 === 0 ? `${h}h` : `${h}h ${mins % 60}m`
}

function SourceBadge({ t }: { t: Task }) {
  const src = sourceMeta[t.source]
  return (
    <Badge variant={src.variant}>
      <src.icon data-icon="inline-start" />
      {src.label}
    </Badge>
  )
}

// Running and Blocked: the full card, because those are the two a person acts on.
function FullCard({ t }: { t: Task }) {
  const agent = t.agentId ? agentById(t.agentId) : undefined
  const blockers = t.deps.map((d) => tasks.find((x) => x.id === d)).filter((x): x is Task => Boolean(x))
  return (
    <motion.div layout="position" transition={layoutSpring} variants={rise}>
      <Card size="sm" className="gap-0">
        <CardContent className="flex flex-col gap-3">
          <p className="text-base leading-snug font-medium">{t.title}</p>

          {/* What the agent itself last said, and how long it has been at it. An agent
              cannot honestly report a completion percentage, so this card does not
              invent one: it prints the agent's own line, which is real. */}
          {t.status === "running" && agent && (
            <div className="border-border bg-canvas flex flex-col gap-1 rounded-lg border p-2.5">
              <p className="text-sm leading-snug">{agent.summary}</p>
              <p className="text-muted-foreground tnum text-xs">running {elapsed(t.createdAt)}</p>
            </div>
          )}

          {t.status === "blocked" && blockers.length > 0 && (
            <div className="border-border bg-canvas flex flex-col gap-1.5 rounded-lg border p-2.5">
              <p className="text-muted-foreground text-xs">Waiting on</p>
              {blockers.map((b) => (
                <p key={b.id} className="flex items-center gap-2 text-xs">
                  <span className="text-muted-foreground font-mono">{b.id}</span>
                  <span className="min-w-0 flex-1 truncate">{b.title}</span>
                  <span className="text-muted-foreground shrink-0">{b.status}</span>
                </p>
              ))}
            </div>
          )}

          <div className="flex flex-wrap items-center gap-1.5">
            <SourceBadge t={t} />
            {agent && (
              <Badge variant="outline" className="gap-1.5">
                <StatusDot status={agent.status} />
                {agent.name}
              </Badge>
            )}
            {!agent && t.status === "blocked" && (
              <span className="text-muted-foreground text-xs">No agent yet</span>
            )}
          </div>

          <p className="text-muted-foreground tnum text-xs">
            {t.id} · added {fmtTime(t.createdAt)}
            {agent && (
              <>
                {" · "}
                <Link to={`/w/${t.workspaceId}/agent/${agent.id}`} className="underline underline-offset-4">
                  open {agent.name}
                </Link>
              </>
            )}
          </p>
        </CardContent>
      </Card>
    </motion.div>
  )
}

// Queued and Done: a line each. They are context, not the thing you came for.
function CompactRow({ t }: { t: Task }) {
  const agent = t.agentId ? agentById(t.agentId) : undefined
  const wait = t.status === "queued"
    ? `waiting ${elapsed(t.createdAt)}`
    : `done ${elapsed(agent?.lastEventAt ?? t.createdAt)} ago`
  return (
    <motion.div layout="position" transition={layoutSpring} variants={rise}>
      <div className="border-border bg-card rounded-lg border px-3 py-2.5">
        <p className={cn("text-sm leading-snug", t.status === "done" && "text-muted-foreground")}>{t.title}</p>
        <div className="mt-1.5 flex flex-wrap items-center gap-x-2 gap-y-1">
          <SourceBadge t={t} />
          {agent && (
            <span className="flex items-center gap-1.5 text-xs">
              <StatusDot status={agent.status} />
              {agent.name}
            </span>
          )}
          <span className="text-muted-foreground tnum ml-auto text-xs">{wait}</span>
        </div>
        {t.status === "done" && agent && (
          <Link to={`/w/${t.workspaceId}/result/${agent.id}`} className="mt-1.5 inline-block text-xs underline underline-offset-4">
            See the result
          </Link>
        )}
      </div>
    </motion.div>
  )
}

function AddTask() {
  return (
    <div className="flex gap-2">
      <Input placeholder="Add a task" aria-label="Add a task to Queued" className="h-8 min-w-0" />
      <Button size="icon-sm" aria-label="Add task" className="shrink-0">
        <Plus />
      </Button>
    </div>
  )
}

function ColumnHead({ label, note, count, lead }: { label: string; note: string; count: number; lead: boolean }) {
  return (
    <div className="flex flex-col gap-0.5">
      <div className="flex items-baseline gap-2">
        <h2 className={cn("u-overline", lead ? "text-foreground" : "text-muted-foreground")}>{label}</h2>
        <BeatCount value={count} className="text-muted-foreground text-xs" />
      </div>
      <p className="text-muted-foreground text-xs">{note}</p>
    </div>
  )
}

function ColumnBody({ list, col }: { list: Task[]; col: (typeof columns)[number] }) {
  const full = col.weight !== "narrow"
  return (
    <motion.div variants={stagger} initial="hidden" animate="show" className="flex flex-col gap-2">
      {col.status === "queued" && <AddTask />}
      {list.map((t) => (full ? <FullCard key={t.id} t={t} /> : <CompactRow key={t.id} t={t} />))}
      {list.length === 0 && (
        <p className="text-muted-foreground border-border rounded-lg border border-dashed px-3 py-4 text-xs leading-relaxed">
          {col.empty}
        </p>
      )}
    </motion.div>
  )
}

export function TasksScreen() {
  const { ws } = useParams()
  const w = workspaces.some((x) => x.id === ws) ? workspaceById(ws!) : workspaces[0]
  const mine = tasks.filter((t) => t.workspaceId === w.id)
  const byStatus = (s: TaskStatus) => mine.filter((t) => t.status === s)
  const [tab, setTab] = useState<TaskStatus>("running")
  const running = byStatus("running").length
  const blocked = byStatus("blocked").length

  return (
    <div className="mx-auto flex w-full min-w-0 flex-col px-4 py-5 md:px-6 md:py-6">
      <header className="flex flex-wrap items-end justify-between gap-x-6 gap-y-2">
        <div className="min-w-0">
          <h1 className="u-display text-3xl md:text-4xl">{w.name}</h1>
          <p className="text-ink-soft mt-1 text-sm">
            <span className="tnum">{running === 1 ? "1 task is running" : `${running} tasks are running`}</span>
            {blocked > 0 && <span className="tnum">, {blocked === 1 ? "1 is blocked behind" : `${blocked} are blocked behind`} them</span>}.
          </p>
        </div>
      </header>

      {/* Weighted by what the column holds, not by how many columns there are. */}
      <motion.div
        variants={stagger}
        initial="hidden"
        animate="show"
        className="mt-6 hidden gap-x-5 gap-y-3 md:grid md:grid-cols-[minmax(0,0.85fr)_minmax(0,1.55fr)_minmax(0,1.3fr)_minmax(0,0.85fr)] md:grid-rows-[auto_auto]"
      >
        {columns.map((c) => (
          <motion.div key={`h-${c.status}`} variants={rise} className="min-w-0">
            <ColumnHead
              label={c.label}
              note={c.note}
              count={byStatus(c.status).length}
              lead={c.weight === "lead"}
            />
          </motion.div>
        ))}
        {columns.map((c) => (
          <motion.div key={`b-${c.status}`} variants={rise} layout="position" transition={layoutSpring} className="min-w-0 self-start">
            <ColumnBody list={byStatus(c.status)} col={c} />
          </motion.div>
        ))}
      </motion.div>

      <Tabs value={tab} onValueChange={(v) => setTab(v as TaskStatus)} className="mt-5 md:hidden">
        <TabsList className="w-full">
          {columns.map((c) => (
            <TabsTrigger key={c.status} value={c.status} className="min-w-0 gap-1 px-1 text-xs">
              <span className="truncate">{c.label}</span>
              <span className="text-muted-foreground tnum">{byStatus(c.status).length}</span>
            </TabsTrigger>
          ))}
        </TabsList>
        <AnimatePresence mode="wait" initial={false}>
          {columns
            .filter((c) => c.status === tab)
            .map((c) => (
              <TabsContent key={c.status} value={c.status} className="pt-3">
                <motion.div variants={swap} initial="hidden" animate="show" exit="exit" className="flex flex-col gap-3">
                  <p className="text-muted-foreground text-xs">{c.note}</p>
                  <ColumnBody list={byStatus(c.status)} col={c} />
                </motion.div>
              </TabsContent>
            ))}
        </AnimatePresence>
      </Tabs>

      <footer className="mt-10">
        <div className="u-seam" />
        <p className="text-muted-foreground mt-3 text-xs">
          <span className="tnum">{mine.length === 1 ? "1 task" : `${mine.length} tasks`} in {w.name}</span>. That is the
          whole board. Pins you drop on the preview arrive here as tasks:{" "}
          <Link to={`/w/${w.id}/preview`} className="underline underline-offset-4">open the preview</Link>.
        </p>
      </footer>
    </div>
  )
}
