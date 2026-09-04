// Screen: / - servers, then their workspaces, then the agents in each.
// The first thing you see on your phone: what needs him, then what every workspace is doing.
import { Link } from "react-router"
import { motion } from "motion/react"
import { ArrowRight, FolderTree, ListTodo, Monitor, RefreshCw, Server as ServerIcon, Sparkles } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Item, ItemActions, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item"
import { Textarea } from "@/components/ui/textarea"
import { StatusBadge, StatusDot } from "@/components/status"
import { agentById, agents, inbox, servers, settings, tasks, workspaces, type Server, type Workspace } from "@/data"
import { rise, stagger } from "@/motion"
import { cn } from "@/lib/utils"

// A stopped agent needs you as much as a question does, so it lands in the same list.
const needsYou = inbox.filter((i) => i.kind === "permission" || i.kind === "question" || i.kind === "failed")

const wsLinks = (id: string) => [
  { to: `/w/${id}/agents`, icon: Sparkles, label: "Agents" },
  { to: `/w/${id}/tasks`, icon: ListTodo, label: "Tasks" },
  { to: `/w/${id}/preview`, icon: Monitor, label: "Preview" },
  { to: `/w/${id}/files`, icon: FolderTree, label: "Files" },
]

function NeedsYouCard() {
  return (
    <Card className="border-destructive/30 ring-destructive/20">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          Needs you
          <Badge variant="destructive">{needsYou.length}</Badge>
        </CardTitle>
        <CardDescription>Answer these and your agents carry on.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        {needsYou.map((i) => (
          <Item key={i.id} variant="outline" className="flex-col items-stretch gap-2 sm:flex-row sm:items-center">
            <ItemContent>
              <ItemTitle className="flex flex-wrap items-center gap-2">
                <span className="min-w-0">{i.title}</span>
                {i.kind === "failed" && <StatusBadge status="failed" />}
              </ItemTitle>
              <ItemDescription>
                {agentById(i.agentId).name} in {i.workspaceId}
              </ItemDescription>
            </ItemContent>
            <ItemActions className="w-full sm:w-auto">
              <Button variant="outline" nativeButton={false} className="h-11 w-full sm:h-8 sm:w-auto" render={<Link to="/inbox" />}>
                Open
                <ArrowRight data-icon="inline-end" />
              </Button>
            </ItemActions>
          </Item>
        ))}
      </CardContent>
    </Card>
  )
}

function WorkspaceCard({ w }: { w: Workspace }) {
  const mine = tasks.filter((t) => t.workspaceId === w.id)
  const counts = [
    { label: "queued", n: mine.filter((t) => t.status === "queued").length },
    { label: "running", n: mine.filter((t) => t.status === "running").length },
    { label: "done", n: mine.filter((t) => t.status === "done").length },
  ]
  return (
    <Card className="h-full">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Link to={`/w/${w.id}/agents`} className="truncate hover:underline">{w.name}</Link>
          <Badge variant="outline">{w.branch}</Badge>
        </CardTitle>
        <CardDescription className="truncate">{w.repo}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-1 flex-col gap-1">
        {w.agents.map((a) => (
          <Link key={a.id} to={`/w/${w.id}/agent/${a.id}`} className="hover:bg-muted flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5">
            <StatusDot status={a.status} />
            <span className="shrink-0 font-medium">{a.name}</span>
            <span className="text-muted-foreground min-w-0 truncate text-xs">{a.summary}</span>
          </Link>
        ))}
      </CardContent>
      <CardFooter className="flex-col items-stretch gap-3">
        <div className="flex flex-wrap gap-1.5">
          {counts.map((c) => (
            <Badge key={c.label} variant="secondary" className="font-normal">
              {c.n} {c.label}
            </Badge>
          ))}
        </div>
        <div className="-mx-1 flex flex-wrap gap-0.5">
          {wsLinks(w.id).map((l) => (
            <Button key={l.to} variant="ghost" size="sm" nativeButton={false} className="text-muted-foreground" render={<Link to={l.to} />}>
              <l.icon data-icon="inline-start" />
              {l.label}
            </Button>
          ))}
        </div>
      </CardFooter>
    </Card>
  )
}

// Servers are the top of the tree (decision 18). One server, no heading; more than one, group under it.
const grouped = servers.length > 1

function ServerHeading({ s, count }: { s: Server; count: number }) {
  const off = s.state !== "online"
  return (
    <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1">
      <ServerIcon className={cn("size-4 shrink-0", off ? "text-muted-foreground" : "text-foreground")} />
      <h2 className={cn("font-heading text-sm font-medium", off && "text-muted-foreground")}>{s.name}</h2>
      {s.home && <Badge variant="secondary">home</Badge>}
      <span className="flex items-center gap-1.5">
        <span className={cn("size-1.5 rounded-full", s.state === "online" ? "bg-primary" : s.state === "installing" ? "bg-primary animate-pulse" : "bg-border")} />
        <span className="text-muted-foreground text-xs">{s.state}</span>
      </span>
      <span className="text-muted-foreground text-xs">· {count} {count === 1 ? "workspace" : "workspaces"}</span>
    </div>
  )
}

function OfflineCard({ s }: { s: Server }) {
  return (
    <Card className="bg-muted/30 border-dashed">
      <CardContent className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-muted-foreground text-sm">{s.name} is offline.</p>
        <Button variant="ghost" size="sm" className="h-9 md:h-8">
          <RefreshCw data-icon="inline-start" />
          Check again
        </Button>
      </CardContent>
    </Card>
  )
}

function ServerSection({ s }: { s: Server }) {
  const mine = workspaces.filter((w) => w.serverId === s.id)
  return (
    <div className="flex flex-col gap-2.5">
      <ServerHeading s={s} count={mine.length} />
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        {mine.map((w) => (
          <motion.div key={w.id} variants={rise} className="min-w-0">
            <WorkspaceCard w={w} />
          </motion.div>
        ))}
        {mine.length === 0 && (
          <motion.div variants={rise} className="min-w-0">
            <OfflineCard s={s} />
          </motion.div>
        )}
      </div>
    </div>
  )
}

function StartCard() {
  return (
    <Card className="h-full">
      <CardHeader>
        <CardTitle>Start something</CardTitle>
        <CardDescription>One sentence is enough. An agent picks it up.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-1 flex-col gap-3 xl:flex-row xl:items-end">
        <Textarea placeholder="Add a Ship button to the result page" className="min-h-20 flex-1" />
        <Button nativeButton={false} className="h-11 w-full sm:h-8 xl:h-11 xl:max-w-40" render={<Link to="/new" />}>
          Start
          <ArrowRight data-icon="inline-end" />
        </Button>
      </CardContent>
    </Card>
  )
}

export function HomeScreen() {
  return (
    <div className="mx-auto w-full max-w-7xl px-4 py-5 md:px-6 md:py-6">
      <div>
        <h1 className="u-display text-3xl md:text-4xl">Good morning, {settings.user.name}</h1>
        <p className="text-muted-foreground text-sm">
          {grouped && `${servers.length} servers, `}{workspaces.length} workspaces, {agents.length} agents, {needsYou.length} things need you
        </p>
      </div>

      <motion.div variants={stagger} initial="hidden" animate="show" className="mt-5 flex flex-col gap-4">
        {needsYou.length > 0 && (
          <motion.div variants={rise}>
            <NeedsYouCard />
          </motion.div>
        )}
        {grouped ? (
          servers.map((s) => (
            <motion.div key={s.id} variants={rise}>
              <ServerSection s={s} />
            </motion.div>
          ))
        ) : (
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
            {workspaces.map((w) => (
              <motion.div key={w.id} variants={rise} className="min-w-0">
                <WorkspaceCard w={w} />
              </motion.div>
            ))}
          </div>
        )}
        <motion.div variants={rise}>
          <StartCard />
        </motion.div>
      </motion.div>
    </div>
  )
}
