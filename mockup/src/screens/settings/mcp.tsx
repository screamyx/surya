// Settings panel: the MCP servers Claude Code has configured. surya reads the config, Claude Code owns it.
import { useState } from "react"
import { Plug, RefreshCw } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Item, ItemActions, ItemContent, ItemDescription, ItemGroup, ItemTitle } from "@/components/ui/item"
import { Separator } from "@/components/ui/separator"
import { Switch } from "@/components/ui/switch"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { mcpServers } from "@/data"
import { cn } from "@/lib/utils"

type McpServer = (typeof mcpServers)[number]

const stateDot: Record<McpServer["state"], string> = {
  connected: "bg-primary",
  "not connected": "bg-destructive",
  disabled: "bg-muted-foreground/40",
}

function StateCell({ s }: { s: McpServer }) {
  return (
    <span className="flex items-center gap-2">
      <span className={cn("size-2 shrink-0 rounded-full", stateDot[s.state])} />
      <span className={cn("text-sm", s.state === "connected" ? "text-foreground" : "text-muted-foreground")}>{s.state}</span>
    </span>
  )
}

function Reconnect({ className }: { className?: string }) {
  return (
    <Button variant="outline" size="sm" className={cn("text-destructive border-destructive/40 hover:bg-destructive/10 hover:text-destructive h-9 px-2.5 text-xs md:h-7 md:px-2", className)}>
      <RefreshCw data-icon="inline-start" />Reconnect
    </Button>
  )
}

export function McpPanel() {
  const [off, setOff] = useState<string[]>(mcpServers.filter((s) => s.state === "disabled").map((s) => s.name))
  const toggle = (name: string, on: boolean) => setOff((prev) => (on ? prev.filter((n) => n !== name) : [...prev, name]))

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2"><Plug className="text-muted-foreground size-4" />MCP servers</CardTitle>
        <CardDescription>What Claude Code's own config says. surya reads it, Claude Code owns it.</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="hidden md:block">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Server</TableHead>
                <TableHead>Transport</TableHead>
                <TableHead>Scope</TableHead>
                <TableHead className="px-1 text-right">Tools</TableHead>
                <TableHead>State</TableHead>
                <TableHead className="w-0 text-right">On</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {mcpServers.map((s) => (
                <TableRow key={s.name}>
                  <TableCell className="py-2.5">
                    <p className="font-mono text-sm font-medium">{s.name}</p>
                  </TableCell>
                  <TableCell><Badge variant="secondary">{s.transport}</Badge></TableCell>
                  <TableCell><Badge variant="outline">{s.scope}</Badge></TableCell>
                  <TableCell className="text-muted-foreground px-1 text-right tabular-nums">{s.tools || "-"}</TableCell>
                  <TableCell>
                    <span className="flex items-center gap-2">
                      <StateCell s={s} />
                      {s.state === "not connected" && <Reconnect />}
                    </span>
                  </TableCell>
                  <TableCell className="text-right">
                    <Switch aria-label={`Enable ${s.name}`} checked={!off.includes(s.name)} onCheckedChange={(on) => toggle(s.name, on)} />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <ItemGroup className="gap-1 md:hidden">
          {mcpServers.map((s) => (
            <Item key={s.name} variant="outline" className="items-start">
              <ItemContent className="min-w-0 gap-1.5">
                <ItemTitle className="font-mono">{s.name}</ItemTitle>
                <ItemDescription className="truncate font-mono">{s.command}</ItemDescription>
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
                  <StateCell s={s} />
                  <Badge variant="secondary">{s.transport}</Badge>
                  <Badge variant="outline">{s.scope}</Badge>
                  <span className="text-muted-foreground text-xs">{s.tools} tools</span>
                </div>
                {s.state === "not connected" && <Reconnect className="mt-1 w-fit" />}
              </ItemContent>
              <ItemActions>
                <Switch aria-label={`Enable ${s.name}`} checked={!off.includes(s.name)} onCheckedChange={(on) => toggle(s.name, on)} />
              </ItemActions>
            </Item>
          ))}
        </ItemGroup>

        <Separator className="my-3" />
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <Button variant="outline" className="h-9 w-fit md:h-8"><Plug data-icon="inline-start" />Add server</Button>
          <p className="text-muted-foreground text-sm">This is what /mcp shows in the terminal. Headless Claude Code has no menu for it, so this screen is it.</p>
        </div>
      </CardContent>
    </Card>
  )
}
