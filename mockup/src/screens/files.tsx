// Screen: files. Feature 7, browse the workspace and watch the agent's edits land.
// Desktop is a resizable tree beside the editor. Phone puts the tree in a sheet.
import { useEffect, useState } from "react"
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable"
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet"
import { fileTree } from "@/data"
import { FileTree } from "@/screens/files/tree"
import { FileEditor, DIFF_PATH } from "@/screens/files/editor"

const openTabs = [
  "backend/app/Models/Lead.php",
  "backend/app/Jobs/SendFollowUpReminder.php",
  "specs/behaviors/crm-lead-follow-up.md",
]

// The shell switches to a sidebar at md, so the editor does the same.
function useIsDesktop() {
  const [isDesktop, setIsDesktop] = useState(() => typeof window !== "undefined" && window.matchMedia("(min-width: 768px)").matches)
  useEffect(() => {
    const mq = window.matchMedia("(min-width: 768px)")
    const onChange = () => setIsDesktop(mq.matches)
    mq.addEventListener("change", onChange)
    return () => mq.removeEventListener("change", onChange)
  }, [])
  return isDesktop
}

export function FilesScreen() {
  const [selected, setSelected] = useState(DIFF_PATH)
  const [mode, setMode] = useState<"edit" | "diff">("edit")
  const [query, setQuery] = useState("")
  const [treeOpen, setTreeOpen] = useState(false)
  const isDesktop = useIsDesktop()

  const open = (path: string) => {
    setSelected(path)
    setTreeOpen(false)
  }

  const editor = (
    <FileEditor
      path={selected}
      tabs={openTabs.includes(selected) ? openTabs : [...openTabs, selected]}
      onSelectTab={setSelected}
      mode={mode}
      onModeChange={setMode}
      isDesktop={isDesktop}
      onOpenTree={() => setTreeOpen(true)}
    />
  )

  return (
    <div className="h-[calc(100svh-3rem-5rem)] md:h-[calc(100svh-3rem)]">
      {isDesktop ? (
        <ResizablePanelGroup orientation="horizontal" className="h-full">
          <ResizablePanel defaultSize={260} minSize={200} maxSize={480} className="min-w-0">
            <FileTree nodes={fileTree} selected={selected} onSelect={open} query={query} onQueryChange={setQuery} />
          </ResizablePanel>
          <ResizableHandle withHandle />
          <ResizablePanel className="min-w-0">{editor}</ResizablePanel>
        </ResizablePanelGroup>
      ) : (
        editor
      )}

      <Sheet open={treeOpen} onOpenChange={setTreeOpen}>
        <SheetContent side="left" className="w-11/12 max-w-sm p-0">
          <SheetHeader className="border-b">
            <SheetTitle>Files</SheetTitle>
            <SheetDescription>project-jag on branch v2</SheetDescription>
          </SheetHeader>
          <FileTree nodes={fileTree} selected={selected} onSelect={open} query={query} onQueryChange={setQuery} className="min-h-0 flex-1" />
        </SheetContent>
      </Sheet>
    </div>
  )
}
