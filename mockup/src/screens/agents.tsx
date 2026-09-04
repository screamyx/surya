// Screen: /w/:ws/agents - feature 3, grouped by operational state.
// Decision 20 and docs/research/synthesis.md: every product that solved this groups by
// state, not by place. Windsurf's Agent Command Center runs "Running / Waiting for
// review / Done"; Cursor runs "IN PROGRESS / READY FOR REVIEW". So do we.
// The spawned-agent tree (decision 15) survives as an indent rail inside each section:
// a child sits under its parent when both share a state, and carries "spawned by raven"
// when it does not. No row is ever drawn twice.
import { useState } from "react"
import { Link, useParams } from "react-router"
import { AnimatePresence, motion } from "motion/react"
import { Check, CornerDownRight, GitBranch, Mail, Plus, Send } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty"
import { Item, ItemActions, ItemContent, ItemDescription } from "@/components/ui/item"
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from "@/components/ui/sheet"
import { Textarea } from "@/components/ui/textarea"
import { useIsMobile } from "@/hooks/use-mobile"
import { deliveryFor } from "@/screens/messages/threads"
import { StatusDot, statusMeta } from "@/components/status"
import {
  agentById, flatten, fmtTime, inbox, now, tasks, workspaceById, workspaces,
  type Agent, type AgentStatus, type Workspace,
} from "@/data"
import { layoutSpring, pop, rise, row, stagger } from "@/motion"
import { cn } from "@/lib/utils"

const action: Record<AgentStatus, { label: string; variant: "default" | "outline" | "secondary" }> = {
  // Answering a permission and retrying a stopped agent are both affirmative, so both
  // wear the primary variant. Destructive is reserved for destroying something.
  "needs-you": { label: "Answer", variant: "default" },
  failed: { label: "Retry", variant: "default" },
  working: { label: "Watch", variant: "outline" },
  done: { label: "See result", variant: "outline" },
  idle: { label: "Give a task", variant: "secondary" },
}

// Four sections, in the order a person reads them: what stops, what runs, what landed,
// what waits. "failed" joins "needs you" because both mean the same thing to you.
const sections: { key: string; label: string; note: string; has: (s: AgentStatus) => boolean }[] = [
  { key: "needs-you", label: "Needs you", note: "Nothing moves here until you answer", has: (s) => s === "needs-you" || s === "failed" },
  { key: "working", label: "Working", note: "Running right now", has: (s) => s === "working" },
  { key: "done", label: "Done", note: "Finished, not shipped", has: (s) => s === "done" },
  { key: "idle", label: "Idle", note: "Will pull the next queued task", has: (s) => s === "idle" },
]

const sessionHref = (a: Agent) => `/w/${a.workspaceId}/agent/${a.id}`

function actionHref(a: Agent) {
  if (a.status === "needs-you" || a.status === "failed") return "/inbox"
  if (a.status === "done") return `/w/${a.workspaceId}/result/${a.id}`
  if (a.status === "idle") return "/new"
  return sessionHref(a)
}

// Minutes since an event, against the mockup's fixed "now". Static: no timer.
function ago(iso: string) {
  const mins = Math.max(0, Math.round((Date.parse(now) - Date.parse(iso)) / 60000))
  if (mins < 1) return "just now"
  if (mins < 60) return `${mins}m ago`
  const h = Math.floor(mins / 60)
  return `${h}h ${mins % 60}m ago`
}

// One flat reading order per section that still shows who spawned whom.
type Entry = { agent: Agent; child: boolean }

function entriesFor(w: Workspace, has: (s: AgentStatus) => boolean): Entry[] {
  const out: Entry[] = []
  for (const a of w.agents) {
    if (has(a.status)) out.push({ agent: a, child: false })
    for (const c of a.children) {
      if (has(c.status)) out.push({ agent: c, child: true })
    }
  }
  return out
}

// Send this agent mail without leaving the list. Decision 19: it lands as its next turn.
function MessageSheet({ agent, open, onOpenChange, onSent }: {
  agent: Agent; open: boolean; onOpenChange: (v: boolean) => void; onSent: () => void
}) {
  const isMobile = useIsMobile()
  const [text, setText] = useState("")
  const send = () => {
    if (!text.trim()) return
    setText("")
    onSent()
  }
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side={isMobile ? "bottom" : "right"} className="gap-0 p-0">
        <SheetHeader className="border-b">
          <SheetTitle>Message {agent.name}</SheetTitle>
          <SheetDescription>{agent.summary}</SheetDescription>
        </SheetHeader>
        <div className="flex-1 p-4">
          <Textarea
            name="compose"
            rows={5}
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={`Message ${agent.name}`}
            className="min-h-32 resize-none"
          />
          <div className="mt-3 space-y-2 rounded-lg border p-3 text-xs">
            <div className="flex items-center justify-between gap-2">
              <span className="text-muted-foreground">Address</span>
              <span className="font-mono">{agent.id}</span>
            </div>
            <div className="flex items-center justify-between gap-2">
              <span className="text-muted-foreground">Delivery id</span>
              <span className="font-mono">{deliveryFor(agent.id)}</span>
            </div>
            <div className="flex items-center justify-between gap-2">
              <span className="text-muted-foreground">Ack</span>
              <span>when the turn that carried it ends</span>
            </div>
          </div>
        </div>
        <SheetFooter className="border-t">
          <p className="text-muted-foreground text-xs">Lands as {agent.name}'s next turn. No polling.</p>
          <Button size="lg" onClick={send} disabled={!text.trim()}>
            <Send data-icon="inline-start" />
            Send
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  )
}

// The mail button and the confirmation line under it. Shared by the card and the row.
function useMail(agent: Agent) {
  const [open, setOpen] = useState(false)
  const [sent, setSent] = useState(false)
  const button = (
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={`Message ${agent.name}`}
      onClick={() => setOpen(true)}
      className="relative z-10 max-sm:size-11"
    >
      <Mail />
    </Button>
  )
  const sheet = <MessageSheet agent={agent} open={open} onOpenChange={setOpen} onSent={() => { setOpen(false); setSent(true) }} />
  // The note is the only thing on this screen that appears after a click, so it enters
  // with the row variant and the neighbours slide down on the same layout spring.
  const note = (
    <AnimatePresence initial={false}>
      {sent && (
        <motion.p
          key="sent"
          variants={row}
          initial="hidden"
          animate="show"
          exit="exit"
          className="text-muted-foreground mt-1.5 flex items-center gap-1.5 overflow-hidden px-3 text-xs"
        >
          <Check className="size-3.5" />
          Sent, delivery {deliveryFor(agent.id)}, lands on {agent.name}'s next turn
        </motion.p>
      )}
    </AnimatePresence>
  )
  return { button, sheet, note }
}

// The one agent that stops you. It gets the size, the ask in full, and the answer.
function LeadCard({ agent, parent }: { agent: Agent; parent?: string }) {
  const ask = inbox.find((i) => i.agentId === agent.id)
  const task = tasks.find((t) => t.agentId === agent.id)
  const act = action[agent.status]
  const mail = useMail(agent)
  return (
    <motion.div layout="position" transition={layoutSpring}>
      <motion.div variants={pop} initial="hidden" animate="show" className="border-border bg-card border-l-destructive shadow-lift rounded-xl border border-l-2 p-4 md:p-5">
        <div className="flex flex-wrap items-center gap-2">
          <StatusDot status={agent.status} />
          <Link to={sessionHref(agent)} className="font-medium underline-offset-4 hover:underline">{agent.name}</Link>
          <Badge variant="outline" className="font-mono text-xs">{agent.model}</Badge>
          {parent && (
            <span className="text-muted-foreground flex items-center gap-1 text-xs">
              <CornerDownRight className="size-3" />
              spawned by {parent}
            </span>
          )}
          <span className="text-muted-foreground tnum ml-auto text-xs">{fmtTime(agent.lastEventAt)} · {ago(agent.lastEventAt)}</span>
        </div>

        <p className="mt-3 max-w-prose text-xl leading-snug">{ask?.title ?? agent.summary}</p>
        {ask && <p className="text-ink-soft mt-1.5 max-w-prose text-sm leading-relaxed">{ask.body}</p>}
        {ask?.command && (
          <p className="bg-canvas text-ink-soft mt-3 overflow-x-auto rounded-md px-3 py-2 font-mono text-xs">{ask.command}</p>
        )}
        {!ask && agent.failure && (
          <p className="text-ink-soft mt-1.5 max-w-prose text-sm leading-relaxed">{agent.failure.detail}</p>
        )}
        {task && (
          <p className="text-muted-foreground mt-3 text-sm">
            On{" "}
            <Link to={`/w/${agent.workspaceId}/tasks`} className="underline underline-offset-4">{task.title}</Link>
          </p>
        )}

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <Button
            variant={act.variant}
            nativeButton={false}
            className="h-11 flex-1 sm:h-9 sm:flex-none"
            render={<Link to={actionHref(agent)} />}
          >
            {act.label}
          </Button>
          <Button variant="outline" nativeButton={false} className="h-11 sm:h-9" render={<Link to={sessionHref(agent)} />}>
            Open session
          </Button>
          {mail.button}
        </div>
      </motion.div>
      {mail.note}
      {mail.sheet}
    </motion.div>
  )
}

// Everything that is not stopping you: one quiet line each.
function AgentRow({ agent }: { agent: Agent }) {
  const task = tasks.find((t) => t.agentId === agent.id)
  const act = action[agent.status]
  const mail = useMail(agent)
  return (
    <motion.div layout="position" transition={layoutSpring}>
      <Item variant="outline" className="relative flex-col items-stretch gap-3 sm:flex-row sm:items-center sm:gap-2.5">
        <div className="flex min-w-0 flex-wrap items-center gap-2 sm:w-56 sm:shrink-0 sm:flex-nowrap">
          <StatusDot status={agent.status} />
          <Link to={sessionHref(agent)} className="truncate font-medium after:absolute after:inset-0">{agent.name}</Link>
          <Badge variant="outline" className="font-mono text-xs">{agent.model}</Badge>
          <span className="text-muted-foreground tnum ml-auto text-xs sm:hidden">{ago(agent.lastEventAt)}</span>
        </div>
        <ItemContent className="gap-0.5">
          <ItemDescription className={cn(agent.status === "idle" && "text-muted-foreground")}>{agent.summary}</ItemDescription>
          {task && <ItemDescription className="text-muted-foreground line-clamp-1">On: {task.title}</ItemDescription>}
        </ItemContent>
        <ItemActions className="w-full shrink-0 sm:w-auto">
          <span className="text-muted-foreground tnum hidden text-xs sm:inline">{ago(agent.lastEventAt)}</span>
          {mail.button}
          <Button
            variant={act.variant}
            nativeButton={false}
            className="relative z-10 h-11 flex-1 sm:h-8 sm:flex-none"
            render={<Link to={actionHref(agent)} />}
          >
            {act.label}
          </Button>
        </ItemActions>
      </Item>
      {mail.note}
      {mail.sheet}
    </motion.div>
  )
}

function Section({ label, note, lead, entries }: { label: string; note: string; lead: boolean; entries: Entry[] }) {
  if (!entries.length) return null
  return (
    <motion.section layout="position" transition={layoutSpring} variants={rise} className="mt-7 first:mt-6">
      <div className="flex flex-wrap items-baseline gap-x-2.5 gap-y-1">
        <h2 className={cn("u-overline", lead ? "text-destructive" : "text-muted-foreground")}>{label}</h2>
        <span className="text-muted-foreground tnum text-xs">{entries.length}</span>
        <span className="text-muted-foreground text-xs">{note}</span>
      </div>
      <div className="u-seam mt-2" />
      <motion.div variants={stagger} initial="hidden" animate="show" className="mt-3 flex flex-col gap-2.5">
        {entries.map((e) => (
          <motion.div
            key={e.agent.id}
            layout="position"
            transition={layoutSpring}
            variants={rise}
            className={cn(e.child && "border-border ml-4 border-l-2 pl-4 md:ml-6")}
          >
            {lead ? (
              <LeadCard agent={e.agent} parent={e.child ? agentById(e.agent.parentId!).name : undefined} />
            ) : (
              <>
                {e.child && (
                  <p className="text-muted-foreground mb-1.5 flex items-center gap-1 text-xs">
                    <CornerDownRight className="size-3" />
                    spawned by {agentById(e.agent.parentId!).name}
                  </p>
                )}
                <AgentRow agent={e.agent} />
              </>
            )}
          </motion.div>
        ))}
      </motion.div>
    </motion.section>
  )
}

export function AgentsScreen() {
  const { ws } = useParams()
  const w = workspaces.some((x) => x.id === ws) ? workspaceById(ws!) : workspaces[0]
  const all = flatten(w.agents)
  const last = all.map((a) => a.lastEventAt).sort().pop()

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col px-4 py-5 md:px-6 md:py-6">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="u-display text-3xl md:text-4xl">{w.name}</h1>
            <Badge variant="outline">{w.branch}</Badge>
          </div>
          <p className="text-muted-foreground text-sm">{w.repo}</p>
          <p className="text-muted-foreground mt-1 font-mono text-xs break-all">{w.worktree}</p>
          {/* The isolation cue Conductor documents: agents in one workspace share branch,
              code state and context, and can edit the same file at the same time. */}
          <p className="text-muted-foreground mt-2 flex max-w-prose items-start gap-1.5 text-xs">
            <GitBranch className="mt-0.5 size-3.5 shrink-0" />
            <span>One worktree, shared. Every agent here works in that directory on {w.branch}, so two of them can edit the same file at the same time.</span>
          </p>
        </div>
        {/* Outline, not primary: the lead action on this screen is the agent that needs
            you, and two filled buttons would fight for the eye. */}
        <Button variant="outline" nativeButton={false} className="h-11 w-full sm:h-9 sm:w-auto" render={<Link to={`/new?ws=${w.id}`} />}>
          <Plus data-icon="inline-start" />
          New ask
        </Button>
      </header>

      {all.length === 0 ? (
        <Empty className="min-h-[60svh]">
          <EmptyHeader>
            <EmptyTitle>No agents in {w.name} yet</EmptyTitle>
            <EmptyDescription>
              Start one with an ask in a sentence. It gets its own name, picks up the branch {w.branch}, and
              reports back here as it works.
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button nativeButton={false} render={<Link to={`/new?ws=${w.id}`} />}>
              <Plus data-icon="inline-start" />
              Start the first agent
            </Button>
          </EmptyContent>
        </Empty>
      ) : (
        <motion.div variants={stagger} initial="hidden" animate="show">
          {sections.map((s) => (
            <Section
              key={s.key}
              label={s.label}
              note={s.note}
              lead={s.key === "needs-you"}
              entries={entriesFor(w, s.has)}
            />
          ))}
        </motion.div>
      )}

      <footer className="mt-10">
        <div className="u-seam" />
        <p className="text-muted-foreground mt-3 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs">
          <span className="tnum">{all.length === 1 ? "1 agent" : `${all.length} agents`} in {w.name}</span>
          {sections.map((s) => {
            const n = all.filter((a) => s.has(a.status)).length
            return n === 0 ? null : (
              <span key={s.key} className="flex items-center gap-1.5">
                <span aria-hidden>·</span>
                <StatusDot status={s.key as AgentStatus} />
                <span className="tnum">{n} {statusMeta[s.key as AgentStatus].label.toLowerCase()}</span>
              </span>
            )
          })}
          {last && <span className="tnum"><span aria-hidden>·</span> last event {fmtTime(last)}</span>}
        </p>
        <p className="text-muted-foreground mt-1 text-xs">That is every agent in this workspace. Nothing is hidden below.</p>
      </footer>
    </div>
  )
}
