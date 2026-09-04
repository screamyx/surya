// Screen: new ask. Feature 1 - pick a workspace, type one sentence, an agent starts.
import { useState } from "react"
import { Link, useSearchParams } from "react-router"
import { motion } from "motion/react"
import { ArrowRight, GitBranch, GitPullRequest, Play, TestTube } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Item, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item"
import { Kbd } from "@/components/ui/kbd"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Textarea } from "@/components/ui/textarea"
import { Toggle } from "@/components/ui/toggle"
import { StatusDot } from "@/components/status"
import { pop, rise, stagger } from "@/motion"
import { agents, serverById, settings, tasks, workspaces } from "@/data"

const modes = ["Build", "Plan only", "Ask questions first"]

const switches = [
  { id: "worktree", label: "Cut a worktree", icon: GitBranch, on: true },
  { id: "pr", label: "Open a PR when done", icon: GitPullRequest, on: true },
  { id: "tests", label: "Run tests", icon: TestTube, on: false },
]

const examples = [
  "Fix the tile jump on upload",
  "Why is the media tab scrolling sideways",
  "Add a Ship button to the result page",
]

// The last three things that were asked, straight off the task board.
const recent = ["t-1", "t-2", "t-4"]
  .map((id) => tasks.find((t) => t.id === id)!)
  .map((t) => ({ task: t, agent: agents.find((a) => a.id === t.agentId)! }))

// A workspace only means something with its server in front of it once there is more than one.
const wsLabel = (id: string) => {
  const w = workspaces.find((x) => x.id === id)
  return w ? `${serverById(w.serverId).name} / ${w.name}` : id
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid gap-1.5">
      <span className="text-muted-foreground text-xs">{label}</span>
      {children}
    </div>
  )
}

export function NewAskScreen() {
  // The rail, the tab bar and the workspace cards all link here as /new?ws=<id>.
  const [params] = useSearchParams()
  const asked = params.get("ws")
  const preset = workspaces.some((w) => w.id === asked) ? asked! : workspaces[0].id

  const [ask, setAsk] = useState("")
  const [ws, setWs] = useState(preset)
  const [on, setOn] = useState<Record<string, boolean>>(Object.fromEntries(switches.map((s) => [s.id, s.on])))
  const picked = workspaces.find((w) => w.id === ws)!

  return (
    <motion.div
      variants={stagger}
      initial="hidden"
      animate="show"
      className="mx-auto w-full max-w-2xl space-y-6 px-4 py-8 md:py-12"
    >
      <motion.div variants={rise} className="space-y-1">
        <h1 className="font-heading text-2xl font-medium tracking-tight">What do you want done?</h1>
        <p className="text-muted-foreground text-sm">One sentence is enough. The agent asks if it needs more.</p>
        {asked && (
          <p className="text-muted-foreground text-sm">
            Starting in {picked.name} on {serverById(picked.serverId).name}
          </p>
        )}
      </motion.div>

      <motion.div variants={rise}>
        <Textarea
          name="ask"
          rows={6}
          value={ask}
          onChange={(e) => setAsk(e.target.value)}
          placeholder="Add follow-up fields to leads, with the reminder the morning after"
          className="min-h-40 text-base leading-relaxed md:text-base"
        />
      </motion.div>

      <motion.div variants={rise} className="grid gap-3 sm:grid-cols-3">
        <Field label="Workspace">
          <Select value={ws} onValueChange={(v) => setWs(v as string)}>
            <SelectTrigger className="w-full"><SelectValue>{(v) => wsLabel(v as string)}</SelectValue></SelectTrigger>
            <SelectContent>
              {workspaces.map((w) => (
                <SelectItem key={w.id} value={w.id}>{wsLabel(w.id)}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
        <Field label="Model">
          <Select defaultValue={settings.models[0]}>
            <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
            <SelectContent>
              {settings.models.map((m) => (
                <SelectItem key={m} value={m}>{m}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
        <Field label="Mode">
          <Select defaultValue={modes[0]}>
            <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
            <SelectContent>
              {modes.map((m) => (
                <SelectItem key={m} value={m}>{m}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
      </motion.div>

      <motion.div variants={rise} className="flex flex-wrap gap-2">
        {switches.map((s) => (
          <Toggle
            key={s.id}
            variant="outline"
            pressed={on[s.id]}
            onPressedChange={(v) => setOn((prev) => ({ ...prev, [s.id]: v }))}
            aria-label={s.label}
            className={on[s.id] ? "border-primary text-foreground" : "text-muted-foreground"}
          >
            <s.icon data-icon="inline-start" />
            {s.label}
          </Toggle>
        ))}
      </motion.div>

      <motion.div variants={rise} className="flex justify-end">
        <Button size="lg" className="w-full sm:w-auto">
          <Play data-icon="inline-start" />
          Start agent
          <Kbd className="bg-primary-foreground/15 text-primary-foreground ml-1">⌘↵</Kbd>
        </Button>
      </motion.div>

      <motion.div variants={rise} className="space-y-2">
        <span className="text-muted-foreground text-xs">Or start from one of these</span>
        <div className="grid gap-2 sm:grid-cols-3">
          {examples.map((x) => (
            <motion.div key={x} variants={pop}>
              <Card
                size="sm"
                role="button"
                tabIndex={0}
                onClick={() => setAsk(x)}
                onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setAsk(x) } }}
                className="hover:bg-muted h-full cursor-pointer transition-colors"
              >
                <CardContent className="text-sm leading-snug">{x}</CardContent>
              </Card>
            </motion.div>
          ))}
        </div>
      </motion.div>

      <motion.div variants={rise} className="space-y-2">
        <span className="text-muted-foreground text-xs">Recent asks</span>
        <div className="grid gap-1.5">
          {recent.map(({ task, agent }) => (
            <Item
              key={task.id}
              variant="outline"
              size="sm"
              render={<Link to={`/w/${task.workspaceId}/agent/${agent.id}`} />}
              className="gap-3"
            >
              <StatusDot status={agent.status} />
              <ItemContent>
                <ItemTitle>{task.title}</ItemTitle>
                <ItemDescription>{agent.name} in {task.workspaceId}</ItemDescription>
              </ItemContent>
              <ArrowRight className="text-muted-foreground size-4 shrink-0" />
            </Item>
          ))}
        </div>
      </motion.div>
    </motion.div>
  )
}
