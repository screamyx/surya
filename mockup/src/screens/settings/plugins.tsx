// Settings panel: plugins and the slash commands they bring, plus every skill an agent can run.
// /plugin is a terminal menu with no headless form, so this screen owns the same files.
import { useState } from "react"
import { Blocks } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Item, ItemActions, ItemContent, ItemDescription, ItemGroup, ItemTitle } from "@/components/ui/item"
import { Separator } from "@/components/ui/separator"
import { Switch } from "@/components/ui/switch"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { plugins, skills } from "@/data"

type SkillEntry = (typeof skills)[number]

const groups: { source: SkillEntry["source"]; label: string }[] = [
  { source: "project", label: "This workspace" },
  { source: "user", label: "Your skills" },
  { source: "plugin", label: "From plugins" },
]

function useToggles(initial: Record<string, boolean>) {
  const [on, setOn] = useState(initial)
  return [on, (name: string, next: boolean) => setOn((prev) => ({ ...prev, [name]: next }))] as const
}

function PluginList() {
  const [on, set] = useToggles(Object.fromEntries(plugins.map((p) => [p.name, p.enabled])))
  return (
    <ItemGroup className="gap-1">
      {plugins.map((p) => (
        <Item key={p.name} variant="outline" className="items-start">
          <ItemContent className="min-w-0 gap-1.5">
            <ItemTitle>{p.name}<Badge variant="secondary">{p.version}</Badge></ItemTitle>
            <ItemDescription className="truncate font-mono">{p.source}</ItemDescription>
            <div className="flex flex-wrap gap-1.5">
              {p.provides.map((c) => (
                <Badge key={c} variant="outline" className="font-mono text-xs font-normal">/{c}</Badge>
              ))}
            </div>
          </ItemContent>
          <ItemActions>
            <Switch aria-label={`Enable ${p.name}`} checked={on[p.name]} onCheckedChange={(v) => set(p.name, v)} />
          </ItemActions>
        </Item>
      ))}
    </ItemGroup>
  )
}

function SkillList() {
  const [on, set] = useToggles(Object.fromEntries(skills.map((s) => [s.name, s.enabled])))
  return (
    <div className="flex flex-col gap-4">
      <p className="text-muted-foreground text-sm">Type / in any agent to use these.</p>
      {groups.map((g) => {
        const list = skills.filter((s) => s.source === g.source)
        if (!list.length) return null
        return (
          <div key={g.source} className="flex flex-col gap-1">
            <p className="text-muted-foreground px-1 text-xs font-medium tracking-wide uppercase">{g.label}</p>
            <ItemGroup className="gap-1">
              {list.map((s) => (
                <Item key={s.name} variant="outline" className="flex-nowrap">
                  <ItemContent className="min-w-0">
                    <ItemTitle className="font-mono">/{s.name}</ItemTitle>
                    <ItemDescription>{s.description}</ItemDescription>
                    <ItemDescription className="truncate font-mono text-xs">{s.path}</ItemDescription>
                  </ItemContent>
                  <ItemActions>
                    <Switch aria-label={`Enable ${s.name}`} checked={on[s.name]} onCheckedChange={(v) => set(s.name, v)} />
                  </ItemActions>
                </Item>
              ))}
            </ItemGroup>
          </div>
        )
      })}
    </div>
  )
}

export function PluginsPanel() {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2"><Blocks className="text-muted-foreground size-4" />Plugins and skills</CardTitle>
        <CardDescription>Bundles of slash commands, and the skills your agents can reach for.</CardDescription>
      </CardHeader>
      <CardContent>
        <Tabs defaultValue="plugins">
          <TabsList className="w-full sm:w-fit">
            <TabsTrigger value="plugins">Plugins<span className="text-muted-foreground">{plugins.length}</span></TabsTrigger>
            <TabsTrigger value="skills">Skills<span className="text-muted-foreground">{skills.length}</span></TabsTrigger>
          </TabsList>
          <TabsContent value="plugins" className="pt-2"><PluginList /></TabsContent>
          <TabsContent value="skills" className="pt-2"><SkillList /></TabsContent>
        </Tabs>
        <Separator className="my-3" />
        <p className="text-muted-foreground text-sm">/plugin has no headless form. surya owns this screen over the same files.</p>
      </CardContent>
    </Card>
  )
}
