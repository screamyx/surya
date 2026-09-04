// Screen: settings. Account, the servers running the daemon, notifications, models, look, workspaces.
import { useEffect, useState } from "react"
import { motion } from "motion/react"
import { Monitor, MoonStar, Plus, Sun } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Item, ItemActions, ItemContent, ItemDescription, ItemGroup, ItemTitle } from "@/components/ui/item"
import { Label } from "@/components/ui/label"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { Separator } from "@/components/ui/separator"
import { Switch } from "@/components/ui/switch"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { McpPanel } from "@/screens/settings/mcp"
import { PluginsPanel } from "@/screens/settings/plugins"
import { ServersPanel } from "@/screens/settings/servers"
import { readTheme, setTheme, type Theme } from "@/screens/settings/theme"
import { serverById, settings, workspaces } from "@/data"
import { rise, stagger } from "@/motion"

function Row({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-baseline justify-between gap-4 py-1.5">
      <span className="text-muted-foreground text-sm">{label}</span>
      <span className={mono ? "truncate font-mono text-sm" : "truncate text-sm font-medium"}>{value}</span>
    </div>
  )
}

function Toggle({ id, label, description, defaultChecked }: { id: string; label: string; description: string; defaultChecked: boolean }) {
  return (
    <div className="flex items-center justify-between gap-4 py-1">
      <Label htmlFor={id} className="flex-col items-start gap-0.5">
        <span className="text-sm font-medium">{label}</span>
        <span className="text-muted-foreground text-sm font-normal">{description}</span>
      </Label>
      <Switch id={id} defaultChecked={defaultChecked} />
    </div>
  )
}

export function SettingsScreen() {
  const [model, setModel] = useState(settings.models[0])
  const [theme, setThemeState] = useState<string[]>(["system"])

  // The saved choice only exists in the browser, so read it after mount and mirror it in the group.
  useEffect(() => setThemeState([readTheme()]), [])
  const pickTheme = (next: string[]) => {
    if (!next.length) return
    setThemeState(next)
    setTheme(next[0] as Theme)
  }

  return (
    <div className="mx-auto w-full max-w-3xl px-4 py-6 md:px-6 md:py-8">
      <header className="flex flex-col gap-1">
        <h1 className="font-heading text-2xl font-semibold tracking-tight md:text-3xl">Settings</h1>
        <p className="text-muted-foreground text-sm">Your account, the machines that run the agents, and how surya reaches you.</p>
      </header>

      <motion.div variants={stagger} initial="hidden" animate="show" className="mt-6 flex flex-col gap-4">
        <motion.div variants={rise}>
          <Card>
            <CardHeader><CardTitle>Account</CardTitle></CardHeader>
            <CardContent>
              <Row label="Name" value={settings.user.name} />
              <Row label="Email" value={settings.user.email} />
              <Row label="Signed in with" value={settings.user.auth} />
              <Separator className="my-3" />
              <Button variant="outline" className="h-9 md:h-8">Sign out</Button>
            </CardContent>
          </Card>
        </motion.div>

        <motion.div variants={rise}><ServersPanel /></motion.div>

        <motion.div variants={rise}><McpPanel /></motion.div>

        <motion.div variants={rise}><PluginsPanel /></motion.div>

        <motion.div variants={rise}>
          <Card>
            <CardHeader><CardTitle>Notifications</CardTitle></CardHeader>
            <CardContent>
              <Toggle id="n-push" label="Push to this phone" description="Turn this off and surya goes quiet everywhere." defaultChecked={settings.notifications.push} />
              <Separator className="my-2" />
              <Toggle id="n-needs" label="When an agent needs you" description="A permission ask or a question that blocks the work." defaultChecked={settings.notifications.needsYou} />
              <Separator className="my-2" />
              <Toggle id="n-results" label="When work is finished" description="One notification per result, with the summary." defaultChecked={settings.notifications.results} />
              <Separator className="my-3" />
              <p className="text-sm font-medium">Quiet hours</p>
              <p className="text-muted-foreground mt-0.5 text-sm">Nothing buzzes between these times. Agents keep working.</p>
              <div className="mt-3 flex flex-wrap items-end gap-3">
                <div className="grid gap-1.5">
                  <Label htmlFor="quiet-from">From</Label>
                  <Input id="quiet-from" type="time" defaultValue={settings.notifications.quietFrom} className="w-32" />
                </div>
                <div className="grid gap-1.5">
                  <Label htmlFor="quiet-to">To</Label>
                  <Input id="quiet-to" type="time" defaultValue={settings.notifications.quietTo} className="w-32" />
                </div>
              </div>
            </CardContent>
          </Card>
        </motion.div>

        <motion.div variants={rise}>
          <Card>
            <CardHeader>
              <CardTitle>Models</CardTitle>
              <CardDescription>Which model a new agent starts on. You can change it per agent.</CardDescription>
            </CardHeader>
            <CardContent>
              <RadioGroup value={model} onValueChange={(v) => setModel(v as string)} className="gap-0">
                {settings.models.map((m) => (
                  <Label key={m} htmlFor={`model-${m}`} className="hover:bg-muted/50 -mx-2 flex items-center gap-3 rounded-lg px-2 py-2.5">
                    <RadioGroupItem id={`model-${m}`} value={m} />
                    <span className="font-mono text-sm font-normal">{m}</span>
                    {m === model && <Badge variant="outline" className="ml-auto">Default</Badge>}
                  </Label>
                ))}
              </RadioGroup>
              <p className="text-muted-foreground mt-3 text-sm">OAuth through your Claude subscription, no API key.</p>
            </CardContent>
          </Card>
        </motion.div>

        <motion.div variants={rise}>
          <Card>
            <CardHeader><CardTitle>Appearance</CardTitle></CardHeader>
            <CardContent>
              <ToggleGroup value={theme} onValueChange={(v) => pickTheme(v as string[])} variant="outline" spacing={0}>
                <ToggleGroupItem value="light"><Sun data-icon="inline-start" />Light</ToggleGroupItem>
                <ToggleGroupItem value="dark"><MoonStar data-icon="inline-start" />Dark</ToggleGroupItem>
                <ToggleGroupItem value="system"><Monitor data-icon="inline-start" />System</ToggleGroupItem>
              </ToggleGroup>
              <p className="text-muted-foreground mt-3 text-sm">Stock shadcn tokens. Dark is the same components with different tokens, nothing else changes.</p>
            </CardContent>
          </Card>
        </motion.div>

        <motion.div variants={rise}>
          <Card>
            <CardHeader>
              <CardTitle>Workspaces</CardTitle>
              <CardDescription>Each one is a folder on a server. Agents are grouped by workspace.</CardDescription>
            </CardHeader>
            <CardContent>
              <ItemGroup className="gap-1">
                {workspaces.map((w) => (
                  <Item key={w.id} variant="outline" className="flex-nowrap">
                    <ItemContent className="min-w-0">
                      <ItemTitle>{w.name}<Badge variant="outline">{w.branch}</Badge></ItemTitle>
                      <ItemDescription className="truncate font-mono">{serverById(w.serverId).name}:{w.worktree}</ItemDescription>
                    </ItemContent>
                    <ItemActions>
                      <Button variant="ghost" size="sm" className="h-9 md:h-7">Remove</Button>
                    </ItemActions>
                  </Item>
                ))}
              </ItemGroup>
              <Separator className="my-3" />
              <div className="flex flex-col gap-2 sm:flex-row">
                <Input placeholder="/home/user/git/new-project" className="font-mono sm:flex-1" aria-label="New workspace path" />
                <Button variant="outline" className="h-9 md:h-8"><Plus data-icon="inline-start" />Add workspace</Button>
              </div>
            </CardContent>
          </Card>
        </motion.div>
      </motion.div>
    </div>
  )
}
