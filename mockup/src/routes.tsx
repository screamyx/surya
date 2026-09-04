// Route table for every surya 1.0 RC surface. Frozen: builders edit their screen files, not this.
import { Routes, Route, Navigate } from "react-router"
import { AppShell } from "@/components/app-shell"
import { WorkspaceLayout } from "@/components/workspace-tabs"
import { HomeScreen } from "@/screens/home"
import { AgentFeedScreen } from "@/screens/agent-feed"
import { InboxScreen } from "@/screens/inbox"
import { AgentsScreen } from "@/screens/agents"
import { PreviewScreen } from "@/screens/preview"
import { TasksScreen } from "@/screens/tasks"
import { ResultScreen } from "@/screens/result"
import { FilesScreen } from "@/screens/files"
import { CatalogScreen } from "@/screens/catalog"
import { NewAskScreen } from "@/screens/new-ask"
import { SettingsScreen } from "@/screens/settings"
import { MessagesScreen } from "@/screens/messages"

export const surfaces = [
  { path: "/", name: "Home", feature: "workspaces overview, agents grouped by workspace" },
  { path: "/new", name: "New ask", feature: "1. Ask in a sentence" },
  { path: "/inbox", name: "Needs you", feature: "2. Needs You inbox with push" },
  { path: "/w/project-jag/agents", name: "Agents", feature: "3. Plain status per agent" },
  { path: "/w/project-jag/agent/raven", name: "Agent feed", feature: "1. Code, chat with tool calls, diffs, cards" },
  { path: "/w/project-jag/preview", name: "Preview", feature: "4. Preview with pins" },
  { path: "/w/project-jag/result/heron", name: "Result", feature: "5. Result you can act on, Ship" },
  { path: "/catalog", name: "Cards", feature: "6. A2UI catalog" },
  { path: "/w/project-jag/files", name: "Files", feature: "7. File tree and editor" },
  { path: "/w/project-jag/tasks", name: "Tasks", feature: "Task board per workspace" },
  { path: "/w/project-jag/messages", name: "Messages", feature: "9. Agent mail: channel and direct threads" },
  { path: "/settings", name: "Settings", feature: "daemon, auth, notifications" },
]

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
            <Route path="preview" element={<PreviewScreen />} />
            <Route path="files" element={<FilesScreen />} />
            <Route path="messages" element={<MessagesScreen />} />
          </Route>
          <Route path="agent/:id" element={<AgentFeedScreen />} />
          <Route path="result/:id" element={<ResultScreen />} />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  )
}
