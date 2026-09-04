// Settings panel: every machine running the surya daemon. Decision 18 - a server is a daemon,
// servers are the top of the tree, and the home server keeps the list.
import { useState } from "react"
import { ArrowUpFromLine, Laptop, Monitor, MonitorSmartphone, Plus, RefreshCw, RotateCw } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Item, ItemActions, ItemContent, ItemGroup, ItemTitle } from "@/components/ui/item"
import { AddServerSheet } from "@/screens/settings/add-server"
import { servers, workspaces, type Server } from "@/data"
import { cn } from "@/lib/utils"

export const osIcon = { linux: Monitor, mac: Laptop, windows: MonitorSmartphone } as const

export const stateDot: Record<Server["state"], string> = {
  online: "bg-primary",
  offline: "bg-muted-foreground/40",
  installing: "bg-primary animate-pulse",
}

export function StateLine({ state }: { state: Server["state"] }) {
  return (
    <span className="flex items-center gap-1.5">
      <span className={cn("size-2 shrink-0 rounded-full", stateDot[state])} />
      <span className={cn("text-sm", state === "offline" ? "text-muted-foreground" : "text-foreground")}>{state}</span>
    </span>
  )
}

function ServerRow({ s }: { s: Server }) {
  const Icon = osIcon[s.os]
  const count = workspaces.filter((w) => w.serverId === s.id).length
  return (
    <Item variant="outline" className="flex-col items-stretch gap-3 sm:flex-row sm:items-center">
      <ItemContent className="min-w-0 gap-2">
        <ItemTitle className="flex flex-wrap items-center gap-2">
          <Icon className="text-muted-foreground size-4 shrink-0" />
          <span className="truncate">{s.name}</span>
          {s.home && <Badge variant="secondary">home</Badge>}
          <span className="text-muted-foreground truncate font-mono text-xs font-normal">{s.host}</span>
        </ItemTitle>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
          <StateLine state={s.state} />
          {s.daemonVersion && <Badge variant="outline" className="font-mono font-normal">surya {s.daemonVersion}</Badge>}
          {s.claudeVersion && <Badge variant="outline" className="font-mono font-normal">claude {s.claudeVersion}</Badge>}
          {s.uptime && <span className="text-muted-foreground text-xs">up {s.uptime}</span>}
          <span className="text-muted-foreground text-xs">{count} {count === 1 ? "workspace" : "workspaces"}</span>
        </div>
      </ItemContent>
      <ItemActions className="justify-start gap-1.5">
        {s.state === "online" && (
          <Button variant="outline" size="sm" className="h-9 md:h-7"><ArrowUpFromLine data-icon="inline-start" />Update</Button>
        )}
        {s.state === "offline" && (
          <Button variant="outline" size="sm" className="h-9 md:h-7"><RefreshCw data-icon="inline-start" />Check again</Button>
        )}
        <Button variant="ghost" size="sm" className="h-9 md:h-7"><RotateCw data-icon="inline-start" />Restart</Button>
      </ItemActions>
    </Item>
  )
}

export function ServersPanel() {
  const [adding, setAdding] = useState(false)
  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="flex flex-col gap-1">
          <CardTitle>Servers</CardTitle>
          <CardDescription>
            A server is a machine running the surya daemon. This app was opened from your home server, which keeps this list.
          </CardDescription>
        </div>
        <Button className="h-9 w-fit shrink-0 md:h-8" onClick={() => setAdding(true)}>
          <Plus data-icon="inline-start" />Add server
        </Button>
      </CardHeader>
      <CardContent>
        <ItemGroup className="gap-2">
          {servers.map((s) => <ServerRow key={s.id} s={s} />)}
        </ItemGroup>
      </CardContent>
      <AddServerSheet open={adding} onOpenChange={setAdding} />
    </Card>
  )
}
