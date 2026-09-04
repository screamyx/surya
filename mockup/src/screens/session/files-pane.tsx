// The files pane. Desktop: the tree beside the editor, or the review reading of the same
// files under the Diff tab. Phone: changed files and the diff only, because the editor and
// the file tree are desktop-only (decision 20, point 4). Cursor's mobile app cuts the same
// way: it "deliberately omits the full editor, terminal, and file browser".
import { useState } from "react"
import { useSearchParams } from "react-router"
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable"
import { fileTree, type Workspace } from "@/data"
import { ChangedFiles } from "@/screens/session/changed-files"
import { FileTree } from "@/screens/session/tree"
import { FileEditor, DIFF_PATH } from "@/screens/session/editor"

const openTabs = [
  "backend/app/Models/Lead.php",
  "backend/app/Jobs/SendFollowUpReminder.php",
  "specs/behaviors/crm-lead-follow-up.md",
]

export function FilesPane({ ws, compact }: { ws: Workspace; compact: boolean }) {
  const [params] = useSearchParams()
  const [selected, setSelected] = useState(DIFF_PATH)
  const [mode, setMode] = useState<"edit" | "diff">("edit")
  const [query, setQuery] = useState("")

  // A phone always gets the review reading. On a desktop the Diff tab asks for it by URL.
  if (compact || params.get("view") === "diff") return <ChangedFiles ws={ws} compact={compact} />

  return (
    <ResizablePanelGroup orientation="horizontal" className="min-h-0 flex-1">
      <ResizablePanel defaultSize={260} minSize={200} maxSize={420} className="min-w-0">
        <FileTree nodes={fileTree} selected={selected} onSelect={setSelected} query={query} onQueryChange={setQuery} />
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel className="min-w-0">
        <FileEditor
          path={selected}
          tabs={openTabs.includes(selected) ? openTabs : [...openTabs, selected]}
          onSelectTab={setSelected}
          mode={mode}
          onModeChange={setMode}
        />
      </ResizablePanel>
    </ResizablePanelGroup>
  )
}
