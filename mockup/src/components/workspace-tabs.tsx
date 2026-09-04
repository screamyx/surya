import { NavLink, Outlet, useParams } from "react-router"
import { ListTodo, Mail, Plus, Sparkles } from "lucide-react"
import { Link } from "react-router"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { workspaceById } from "@/data"

// The three pages every workspace has. Files and Preview left this bar in decision 20:
// they are panes of an agent session now, not places of their own.
export function WorkspaceLayout() {
  const { ws } = useParams()
  const w = workspaceById(ws!)
  const tabs = [
    { to: `/w/${w.id}/agents`, icon: Sparkles, label: "Agents" },
    { to: `/w/${w.id}/tasks`, icon: ListTodo, label: "Tasks" },
    { to: `/w/${w.id}/messages`, icon: Mail, label: "Messages" },
  ]
  return (
    <>
      <div className="bg-background/95 sticky top-12 z-20 flex items-center gap-1 overflow-x-auto border-b px-3 backdrop-blur">
        {tabs.map((t) => (
          <NavLink key={t.to} to={t.to} className={({ isActive }) => cn("text-muted-foreground -mb-px flex shrink-0 items-center gap-1.5 border-b-2 border-transparent px-3 py-2.5 text-sm", isActive && "border-primary text-foreground font-medium")}>
            <t.icon className="size-4" />
            {t.label}
          </NavLink>
        ))}
        <Button size="sm" variant="outline" nativeButton={false} render={<Link to={`/new?ws=${w.id}`} />} className="ml-auto shrink-0">
          <Plus data-icon="inline-start" />
          <span className="hidden sm:inline">New agent</span>
        </Button>
      </div>
      <Outlet />
    </>
  )
}
