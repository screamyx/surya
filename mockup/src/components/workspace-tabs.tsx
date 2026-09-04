import { NavLink, Outlet, useParams } from "react-router"
import { FolderTree, ListTodo, Monitor, Sparkles } from "lucide-react"
import { cn } from "@/lib/utils"
import { workspaceById } from "@/data"

// The four pages every workspace has. They live here, on the page, not in the rail.
export function WorkspaceLayout() {
  const { ws } = useParams()
  const w = workspaceById(ws!)
  const tabs = [
    { to: `/w/${w.id}/agents`, icon: Sparkles, label: "Agents" },
    { to: `/w/${w.id}/tasks`, icon: ListTodo, label: "Tasks" },
    { to: `/w/${w.id}/preview`, icon: Monitor, label: "Preview" },
    { to: `/w/${w.id}/files`, icon: FolderTree, label: "Files" },
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
      </div>
      <Outlet />
    </>
  )
}
