// Route table for every surya 1.0 RC surface.
// Decision 20 folded Feed, Files and Preview into one session surface: they are panes
// now, and a pane is linkable. Result stays separate, for the ship decision only.
import { Routes, Route, Navigate, useParams } from "react-router"
import { AppShell } from "@/components/app-shell"
import { WorkspaceLayout } from "@/components/workspace-tabs"
import { HomeScreen } from "@/screens/home"
import { SessionScreen } from "@/screens/session"
import { InboxScreen } from "@/screens/inbox"
import { AgentsScreen } from "@/screens/agents"
import { TasksScreen } from "@/screens/tasks"
import { ResultScreen } from "@/screens/result"
import { CatalogScreen } from "@/screens/catalog"
import { NewAskScreen } from "@/screens/new-ask"
import { SettingsScreen } from "@/screens/settings"
import { MessagesScreen } from "@/screens/messages"
import { byPriority, workspaceById } from "@/data"

export const surfaces = [
  { path: "/", name: "Home", feature: "the attention queue: what needs you, what is running, what is done" },
  { path: "/new", name: "New ask", feature: "1. Ask in a sentence" },
  { path: "/inbox", name: "Needs you", feature: "2. Needs You inbox with push, and always-allow rules" },
  { path: "/w/project-jag/agents", name: "Agents", feature: "3. Plain status per agent, grouped by state" },
  { path: "/w/project-jag/agent/raven", name: "Agent feed", feature: "1. The session: transcript pane" },
  { path: "/w/project-jag/agent/raven/preview", name: "Preview", feature: "4. The session: preview pane, with pins" },
  { path: "/w/project-jag/result/heron", name: "Result", feature: "5. Result you can act on, Ship" },
  { path: "/catalog", name: "Cards", feature: "6. A2UI catalog" },
  { path: "/w/project-jag/agent/raven/files", name: "Files", feature: "7. The session: files pane, tree and editor" },
  { path: "/w/project-jag/tasks", name: "Tasks", feature: "Task board per workspace" },
  { path: "/w/project-jag/messages", name: "Messages", feature: "9. Agent mail: channel and direct threads" },
  { path: "/settings", name: "Settings", feature: "daemon, auth, notifications, approval policy" },
]

// The old top-level Files and Preview links still resolve. They land on the pane of
// whichever agent in this workspace most needs looking at.
function LeadAgentPane({ pane }: { pane: "files" | "preview" }) {
  const { ws } = useParams()
  const id = ws ?? "project-jag"
  const lead = byPriority(workspaceById(id).agents)[0]
  return <Navigate to={`/w/${id}/agent/${lead.id}/${pane}`} replace />
}

export function AppRoutes() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<HomeScreen />} />
        <Route path="new" element={<NewAskScreen />} />
        <Route path="inbox" element={<InboxScreen />} />
        <Route path="catalog" element={<CatalogScreen />} />
        <Route path="settings" element={<SettingsScreen />} />
        <Route path="w/:ws">
          <Route index element={<Navigate to="agents" replace />} />
          <Route element={<WorkspaceLayout />}>
            <Route path="agents" element={<AgentsScreen />} />
            <Route path="tasks" element={<TasksScreen />} />
            <Route path="messages" element={<MessagesScreen />} />
          </Route>
          <Route path="agent/:id">
            <Route index element={<SessionScreen pane="transcript" />} />
            <Route path="files" element={<SessionScreen pane="files" />} />
            <Route path="preview" element={<SessionScreen pane="preview" />} />
          </Route>
          <Route path="files" element={<LeadAgentPane pane="files" />} />
          <Route path="preview" element={<LeadAgentPane pane="preview" />} />
          <Route path="result/:id" element={<ResultScreen />} />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  )
}
