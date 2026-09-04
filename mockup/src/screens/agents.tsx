// Screen: /w/:ws/agents - feature 3, plain status per agent.
// Working, needs you, done, idle, each with one line a person can read.
import { Link, useParams } from "react-router"
import { motion } from "motion/react"
import { Plus } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Item, ItemActions, ItemContent, ItemDescription } from "@/components/ui/item"
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

function AgentRow({ agent }: { agent: Agent }) {
  const task = tasks.find((t) => t.agentId === agent.id)
  const act = action[agent.status]
  return (
    <Item
      variant="outline"
      className="relative flex-col items-stretch gap-3 sm:flex-row sm:items-center sm:gap-2.5"
    >
      <div className="flex items-center gap-2 sm:w-60 sm:shrink-0">
        <StatusDot status={agent.status} />
        <Link to={`/w/${agent.workspaceId}/agent/${agent.id}`} className="font-medium after:absolute after:inset-0">
          {agent.name}
        </Link>
        <Badge variant="outline" className="font-mono text-xs">{agent.model}</Badge>
        <span className="text-muted-foreground ml-auto text-xs tabular-nums sm:hidden">{fmtTime(agent.lastEventAt)}</span>
      </div>
      <ItemContent className="gap-0.5">
        <ItemDescription className={cn(agent.status === "idle" && "text-muted-foreground/70")}>
          {agent.summary}
        </ItemDescription>
        {task && <ItemDescription className="text-muted-foreground/70 line-clamp-1">On: {task.title}</ItemDescription>}
      </ItemContent>
      <ItemActions className="w-full shrink-0 sm:w-auto">
        <span className="text-muted-foreground hidden text-xs tabular-nums sm:inline">{fmtTime(agent.lastEventAt)}</span>
        <Button
          variant={act.variant}
          nativeButton={false}
          className="relative z-10 h-11 w-full sm:h-8 sm:w-auto"
          render={<Link to={actionHref(agent)} />}
        >
          {act.label}
        </Button>
      </ItemActions>
    </Item>
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
            <h1 className="font-heading text-xl font-semibold tracking-tight">{w.name}</h1>
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
