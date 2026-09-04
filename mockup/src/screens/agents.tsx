// Screen: /w/:ws/agents - feature 3, plain status per agent.
// Working, needs you, done, idle, each with one line a person can read.
import { useState } from "react"
import { Link, useParams } from "react-router"
import { motion } from "motion/react"
import { Check, Mail, Plus, Send } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Item, ItemActions, ItemContent, ItemDescription } from "@/components/ui/item"
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from "@/components/ui/sheet"
import { Textarea } from "@/components/ui/textarea"
import { useIsMobile } from "@/hooks/use-mobile"
import { deliveryFor } from "@/screens/messages/threads"
import { StatusDot } from "@/components/status"
import { flatten, fmtTime, tasks, workspaceById, workspaces, type Agent, type AgentStatus } from "@/data"
import { rise, stagger } from "@/motion"
import { cn } from "@/lib/utils"

const action: Record<AgentStatus, { label: string; variant: "default" | "outline" | "destructive" | "secondary" }> = {
  "needs-you": { label: "Answer", variant: "destructive" },
  failed: { label: "Retry", variant: "destructive" },
  working: { label: "Watch", variant: "outline" },
  done: { label: "See result", variant: "outline" },
  idle: { label: "Give a task", variant: "secondary" },
}

const tiles: { status: AgentStatus; label: string }[] = [
  { status: "working", label: "Working" },
  { status: "needs-you", label: "Needs you" },
  { status: "done", label: "Done" },
]

function actionHref(a: Agent) {
  if (a.status === "needs-you") return "/inbox"
  if (a.status === "done") return `/w/${a.workspaceId}/result/${a.id}`
  if (a.status === "idle") return "/new"
  return `/w/${a.workspaceId}/agent/${a.id}`
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

function AgentRow({ agent }: { agent: Agent }) {
  const task = tasks.find((t) => t.agentId === agent.id)
  const act = action[agent.status]
  const [mailOpen, setMailOpen] = useState(false)
  const [sent, setSent] = useState(false)
  return (
    <>
    <Item
      variant="outline"
      className="relative flex-col items-stretch gap-3 sm:flex-row sm:items-center sm:gap-2.5"
    >
      <div className="flex min-w-0 flex-wrap items-center gap-2 sm:w-60 sm:flex-nowrap sm:shrink-0">
        <StatusDot status={agent.status} />
        <Link to={`/w/${agent.workspaceId}/agent/${agent.id}`} className="font-medium after:absolute after:inset-0">
          {agent.name}
        </Link>
        <Badge variant="outline" className="font-mono text-xs">{agent.model}</Badge>
        <span className="text-muted-foreground ml-auto text-xs tabular-nums sm:hidden">{fmtTime(agent.lastEventAt)}</span>
      </div>
      <ItemContent className="gap-0.5">
        <ItemDescription className={cn(agent.status === "idle" && "text-muted-foreground")}>
          {agent.summary}
        </ItemDescription>
        {task && <ItemDescription className="text-muted-foreground line-clamp-1">On: {task.title}</ItemDescription>}
      </ItemContent>
      <ItemActions className="w-full shrink-0 sm:w-auto">
        <span className="text-muted-foreground hidden text-xs tabular-nums sm:inline">{fmtTime(agent.lastEventAt)}</span>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={`Message ${agent.name}`}
          onClick={() => setMailOpen(true)}
          className="relative z-10 max-sm:size-11"
        >
          <Mail />
        </Button>
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
    {sent && (
      <p className="text-muted-foreground mt-1.5 flex items-center gap-1.5 px-3 text-xs">
        <Check className="size-3.5" />
        Sent, delivery {deliveryFor(agent.id)}, lands on {agent.name}'s next turn
      </p>
    )}
    <MessageSheet agent={agent} open={mailOpen} onOpenChange={setMailOpen} onSent={() => { setMailOpen(false); setSent(true) }} />
    </>
  )
}

export function AgentsScreen() {
  const { ws } = useParams()
  const w = workspaces.some((x) => x.id === ws) ? workspaceById(ws!) : workspaces[0]

  return (
    <div className="mx-auto w-full max-w-5xl px-4 py-5 md:px-6 md:py-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="u-display text-3xl md:text-4xl">{w.name}</h1>
            <Badge variant="outline">{w.branch}</Badge>
          </div>
          <p className="text-muted-foreground text-sm">{w.repo}</p>
          <p className="text-muted-foreground mt-1 font-mono text-xs break-all">{w.worktree}</p>
        </div>
        <Button nativeButton={false} className="h-11 w-full sm:h-8 sm:w-auto" render={<Link to="/new" />}>
          <Plus data-icon="inline-start" />
          New ask
        </Button>
      </div>

      <div className="mt-5 grid grid-cols-3 gap-3">
        {tiles.map((t) => (
          <Card key={t.status} size="sm" className="items-center text-center">
            <div className="flex flex-col items-center gap-1">
              <span className="font-heading text-2xl leading-none font-semibold tabular-nums">
                {flatten(w.agents).filter((a) => a.status === t.status).length}
              </span>
              <span className="text-muted-foreground flex items-center gap-1.5 text-xs">
                <StatusDot status={t.status} />
                {t.label}
              </span>
            </div>
          </Card>
        ))}
      </div>

      <motion.div variants={stagger} initial="hidden" animate="show" className="mt-4 flex flex-col gap-2.5">
        {w.agents.map((a) => (
          <motion.div key={a.id} variants={rise}>
            <AgentRow agent={a} />
            {a.children.length > 0 && (
              <div className="border-border ml-5 mt-1.5 flex flex-col gap-1.5 border-l-2 pl-3">
                <div className="text-muted-foreground text-xs">spawned by {a.name}</div>
                {a.children.map((c) => <AgentRow key={c.id} agent={c} />)}
              </div>
            )}
          </motion.div>
        ))}
      </motion.div>
    </div>
  )
}
