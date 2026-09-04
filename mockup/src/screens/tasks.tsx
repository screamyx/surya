// Screen: tasks. The task board per workspace. Agents pull from Queued when they go idle.
import { useState } from "react"
import { Link, useParams } from "react-router"
import { motion } from "motion/react"
import { MapPin, Plus, Sparkles, User } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Progress } from "@/components/ui/progress"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { StatusDot } from "@/components/status"
import { agentById, fmtTime, tasks, workspaceById, type Task, type TaskStatus } from "@/data"
import { rise, stagger } from "@/motion"
import { cn } from "@/lib/utils"

const columns: { status: TaskStatus; label: string }[] = [
  { status: "queued", label: "Queued" },
  { status: "running", label: "Running" },
  { status: "blocked", label: "Blocked" },
  { status: "done", label: "Done" },
]

const sourceMeta = {
  you: { label: "You", icon: User, variant: "outline" as const },
  pin: { label: "From a pin", icon: MapPin, variant: "secondary" as const },
  agent: { label: "Agent", icon: Sparkles, variant: "secondary" as const },
}

// Static progress per running task. No timers: the mockup is a photograph, not a film.
const progressOf: Record<string, number> = { "t-1": 72, "t-2": 45, "t-6": 60 }

function TaskCard({ t }: { t: Task }) {
  const src = sourceMeta[t.source]
  const agent = t.agentId ? agentById(t.agentId) : undefined
  return (
    <motion.div variants={rise}>
      <Card size="sm" className="gap-2">
        <CardContent className="flex flex-col gap-2">
          <p className={cn("text-sm leading-snug font-medium", t.status === "done" && "text-muted-foreground")}>
            {t.title}
          </p>
          {t.status === "running" && <Progress value={progressOf[t.id] ?? 50} className="my-0.5 gap-0" />}
          <div className="flex flex-wrap items-center gap-1.5">
            <Badge variant={src.variant}>
              <src.icon data-icon="inline-start" />
              {src.label}
            </Badge>
            {agent && (
              <Badge variant="outline" className="gap-1.5">
                <StatusDot status={agent.status} />
                {agent.name}
              </Badge>
            )}
            {t.deps.map((d) => (
              <Badge key={d} variant="outline" className="text-muted-foreground font-mono">
                after {d}
              </Badge>
            ))}
          </div>
          <p className="text-muted-foreground text-xs">
            {t.id} · added {fmtTime(t.createdAt)}
          </p>
        </CardContent>
      </Card>
    </motion.div>
  )
}

function AddTask() {
  return (
    <div className="flex gap-2">
      <Input placeholder="Add a task" className="h-8" />
      <Button size="icon-sm" aria-label="Add task">
        <Plus />
      </Button>
    </div>
  )
}

function ColumnBody({ list, status, className }: { list: Task[]; status: TaskStatus; className?: string }) {
  return (
    <motion.div
      variants={stagger}
      initial="hidden"
      animate="show"
      className={cn("bg-muted/40 flex flex-col gap-2 rounded-xl p-2", className)}
    >
      {status === "queued" && <AddTask />}
      {list.map((t) => (
        <TaskCard key={t.id} t={t} />
      ))}
      {list.length === 0 && (
        <p className="text-muted-foreground rounded-lg border border-dashed p-4 text-center text-xs">
          Nothing here.
        </p>
      )}
    </motion.div>
  )
}

export function TasksScreen() {
  const { ws } = useParams()
  const w = workspaceById(ws ?? "project-jag")
  const mine = tasks.filter((t) => t.workspaceId === w.id)
  const byStatus = (s: TaskStatus) => mine.filter((t) => t.status === s)
  const [tab, setTab] = useState<TaskStatus>("running")

  return (
    <div className="flex min-w-0 flex-col gap-4 p-4">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <div className="flex items-baseline gap-2">
          <h1 className="text-base font-medium">{w.name}</h1>
          <span className="text-muted-foreground text-sm">{mine.length} tasks</span>
        </div>
        <p className="text-muted-foreground text-sm">Agents pull from Queued when they go idle.</p>
      </div>

      <div className="hidden gap-4 md:grid md:grid-cols-4">
        {columns.map((c) => (
          <div key={c.status} className="flex min-w-0 flex-col gap-2">
            <div className="flex items-center gap-2">
              <h2 className="text-sm font-medium">{c.label}</h2>
              <Badge variant="secondary">{byStatus(c.status).length}</Badge>
            </div>
            <ColumnBody list={byStatus(c.status)} status={c.status} className="min-h-96" />
          </div>
        ))}
      </div>

      <Tabs value={tab} onValueChange={(v) => setTab(v as TaskStatus)} className="md:hidden">
        <TabsList className="w-full">
          {columns.map((c) => (
            <TabsTrigger key={c.status} value={c.status} className="min-w-0 gap-1 px-1 text-xs">
              <span className="truncate">{c.label}</span>
              <span className="text-muted-foreground">{byStatus(c.status).length}</span>
            </TabsTrigger>
          ))}
        </TabsList>
        {columns.map((c) => (
          <TabsContent key={c.status} value={c.status} className="pt-1">
            <ColumnBody list={byStatus(c.status)} status={c.status} />
          </TabsContent>
        ))}
      </Tabs>

      <p className="text-muted-foreground text-xs md:hidden">
        Tasks also arrive from pins.{" "}
        <Link to={`/w/${w.id}/preview`} className="underline underline-offset-4">
          Open the preview
        </Link>
      </p>
    </div>
  )
}
