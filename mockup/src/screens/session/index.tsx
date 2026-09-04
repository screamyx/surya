// The agent session: one surface with panes (decision 20, point 2).
// Transcript on the left, and the files, diff or live preview on the right.
// Feed, Files and Preview used to be three top-level routes; they are panes now, because
// four of the ten products researched keep review on one split surface: Codex "keeps the
// conversation, generated visual result, file list, and diff in one split review surface".
import { useEffect, useRef, useState } from "react"
import { useNavigate, useParams, useSearchParams } from "react-router"
import { FileDiff, FolderTree, Monitor, MessageSquare, PanelRightClose, PanelRightOpen } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { usePanelRef } from "react-resizable-panels"
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable"
import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { cn } from "@/lib/utils"
import { agents, agentById, workspaceById } from "@/data"
import { fade, swap } from "@/motion"
import { SessionHeader } from "@/screens/session/header"
import { ComposeProvider } from "@/screens/session/compose"
import { SessionControlsProvider, useSessionControls } from "@/screens/session/controls"
import { ModelSheet } from "@/screens/session/model-sheet"
import { SessionsSheet } from "@/screens/session/sessions-sheet"
import { StopDialog } from "@/screens/session/stop-dialog"
import { Transcript } from "@/screens/session/transcript"
import { FilesPane } from "@/screens/session/files-pane"
import { PreviewPane } from "@/screens/session/preview-pane"

export type SessionPane = "transcript" | "files" | "preview"

function useMedia(query: string) {
  const [matches, setMatches] = useState(() => typeof window !== "undefined" && window.matchMedia(query).matches)
  useEffect(() => {
    const mq = window.matchMedia(query)
    const onChange = () => setMatches(mq.matches)
    mq.addEventListener("change", onChange)
    return () => mq.removeEventListener("change", onChange)
  }, [query])
  return matches
}

// Two thresholds, because they answer different questions.
// The shell switches to a sidebar at md, so below that this is a phone: one pane at a
// time, and the files pane drops the tree and the editor (decision 20, point 4).
export const useIsDesktop = () => useMedia("(min-width: 768px)")
// Below this the split has no room: the rail takes 256px and a three-way split leaves the
// tree and the editor a few characters each. So one pane at a time, at full width.
const useIsWide = () => useMedia("(min-width: 1024px)")

// Three tabs on the right of a desktop split, and a pane is linkable: Files and Diff are
// two readings of the same route, Preview is its own.
type RightTab = "files" | "diff" | "preview"
const rightTabs: { value: RightTab; label: string; icon: typeof FolderTree; to: (base: string) => string }[] = [
  { value: "files", label: "Files", icon: FolderTree, to: (b) => `${b}/files` },
  { value: "diff", label: "Diff", icon: FileDiff, to: (b) => `${b}/files?view=diff` },
  { value: "preview", label: "Preview", icon: Monitor, to: (b) => `${b}/preview` },
]

// One pane at a time. A phone gets three: its files pane is already the diff reading.
// A narrow desktop gets four, because there the files pane is still the tree and the editor.
type Stacked = "transcript" | "files" | "diff" | "preview"
const stackedTabs: { value: Stacked; label: string; icon: typeof FolderTree; to: (base: string) => string }[] = [
  { value: "transcript", label: "Transcript", icon: MessageSquare, to: (b) => b },
  { value: "files", label: "Files", icon: FolderTree, to: (b) => `${b}/files` },
  { value: "diff", label: "Diff", icon: FileDiff, to: (b) => `${b}/files?view=diff` },
  { value: "preview", label: "Preview", icon: Monitor, to: (b) => `${b}/preview` },
]

function PaneTabs({ tab, base, onCollapse }: { tab: RightTab; base: string; onCollapse: () => void }) {
  const navigate = useNavigate()
  return (
    <div className="bg-background flex shrink-0 items-center gap-1 border-b px-2 py-1.5">
      <ToggleGroup
        value={[tab]}
        onValueChange={(v) => {
          const next = rightTabs.find((t) => t.value === v[0])
          if (next) navigate(next.to(base))
        }}
        variant="outline"
        size="sm"
        spacing={0}
        aria-label="Pane"
      >
        {rightTabs.map((t) => (
          <ToggleGroupItem key={t.value} value={t.value}>
            <t.icon data-icon="inline-start" />
            {t.label}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
      <Tooltip>
        <TooltipTrigger
          render={
            <Button variant="ghost" size="icon-sm" className="ml-auto" onClick={onCollapse} aria-label="Close the side pane">
              <PanelRightClose />
            </Button>
          }
        />
        <TooltipContent side="left">Close the pane</TooltipContent>
      </Tooltip>
    </div>
  )
}

function StackedTabs({ tab, base, compact }: { tab: Stacked; base: string; compact: boolean }) {
  const navigate = useNavigate()
  const shown = compact ? stackedTabs.filter((t) => t.value !== "diff") : stackedTabs
  return (
    <div className="bg-background shrink-0 border-b px-4 py-2">
      <ToggleGroup
        value={[tab]}
        onValueChange={(v) => {
          const next = stackedTabs.find((t) => t.value === v[0])
          if (next) navigate(next.to(base))
        }}
        variant="outline"
        size="sm"
        spacing={0}
        className="w-full"
        aria-label="Pane"
      >
        {shown.map((t) => (
          <ToggleGroupItem key={t.value} value={t.value} className="min-w-0 flex-1">
            <t.icon data-icon="inline-start" />
            {t.label}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
    </div>
  )
}

// The overlays the header drives. They sit in the shell so Stop and Sessions keep working
// while the phone is showing the files pane and the transcript is not mounted.
function SessionOverlays({ name, sessions }: { name: string; sessions: Parameters<typeof SessionsSheet>[0]["sessions"] }) {
  const c = useSessionControls()
  return (
    <>
      <ModelSheet open={c.modelOpen} onOpenChange={c.setModelOpen} model={c.model} effort={c.effort} onSwitch={c.switchModel} />
      <SessionsSheet open={c.sessionsOpen} onOpenChange={c.setSessionsOpen} name={name} sessions={sessions} />
      <StopDialog open={c.stopOpen} onOpenChange={c.setStopOpen} name={name} onConfirm={c.stop} />
    </>
  )
}

function SessionBody({ pane, base }: { pane: SessionPane; base: string }) {
  const { ws, id } = useParams()
  const workspace = workspaceById(ws ?? "project-jag")
  // Only raven carries a transcript in the mockup, so an unknown id lands on the first agent.
  const agent = agents.find((a) => a.id === id) ?? agentById(workspace.agents[0].id)
  const controls = useSessionControls()
  const isDesktop = useIsDesktop()
  const wide = useIsWide()
  const [params] = useSearchParams()
  const diff = params.get("view") === "diff"
  const rightTab: RightTab = pane === "preview" ? "preview" : diff ? "diff" : "files"
  // The pane is closed by hand and reopens the moment a pane is asked for by URL, so a
  // pin sending you to Preview never lands on a pane you cannot see.
  const [paneClosed, setPaneClosed] = useState(false)
  const paneRef = usePanelRef()
  // The width is eased by default so collapsing and reopening glide. It is switched
  // off for the duration of a drag, because a lagging handle feels broken, not smooth.
  const [dragging, setDragging] = useState(false)
  useEffect(() => {
    if (!dragging) return
    const stop = () => setDragging(false)
    window.addEventListener("pointerup", stop)
    return () => window.removeEventListener("pointerup", stop)
  }, [dragging])
  const wanted = useRef(pane)
  useEffect(() => {
    if (wanted.current !== pane) { wanted.current = pane; if (pane !== "transcript") paneRef.current?.expand() }
  }, [pane])
  const stacked: Stacked = pane === "files" && diff && isDesktop ? "diff" : pane

  return (
    <div className="flex h-[calc(100svh-3rem-5rem)] min-w-0 flex-col md:h-[calc(100svh-3rem)]">
      <SessionHeader
        agent={agent}
        ws={workspace}
        model={controls.model}
        stopped={controls.stopped}
        onStop={() => controls.setStopOpen(true)}
        onResume={controls.resume}
        onSessions={() => controls.setSessionsOpen(true)}
      />

      {wide ? (
        <ResizablePanelGroup orientation="horizontal" className={cn("min-h-0 flex-1", !dragging && "pane-easing")}>
          <ResizablePanel defaultSize="46%" minSize="30%" className="relative flex min-w-0 flex-col">
            <Transcript agent={agent} ws={workspace} />
            {/* When the pane is closed the transcript takes the whole width, and this is
                the only way back to it. Owner, 04:57: the pane has to be collapsable. */}
            <AnimatePresence>
              {paneClosed && (
              <motion.div variants={fade} initial="hidden" animate="show" exit="hidden" className="absolute top-2 right-3 z-10">
              <Tooltip>
                <TooltipTrigger
                  render={
                    <Button
                      variant="outline"
                      size="icon-sm"
                      className="bg-background"
                      onClick={() => paneRef.current?.expand()}
                      aria-label="Open the side pane"
                    >
                      <PanelRightOpen />
                    </Button>
                  }
                />
                <TooltipContent side="left">Open the pane</TooltipContent>
              </Tooltip>
              </motion.div>
              )}
            </AnimatePresence>
          </ResizablePanel>
          <ResizableHandle withHandle onPointerDown={() => setDragging(true)} className={paneClosed ? "pointer-events-none opacity-0" : undefined} />
          <ResizablePanel
            panelRef={paneRef}
            collapsible
            collapsedSize={0}
            minSize="30%"
            onResize={(size) => setPaneClosed(size.asPercentage < 1)}
            className="bg-canvas flex min-w-0 flex-col overflow-hidden"
          >
                <PaneTabs tab={rightTab} base={base} onCollapse={() => paneRef.current?.collapse()} />
                {/* The pane content cross-fades between Files, Diff and Preview: the one
                    leaving goes before the one arriving, so nothing overlaps mid-swap. */}
                <AnimatePresence mode="wait" initial={false}>
                  <motion.div
                    key={rightTab}
                    variants={swap}
                    initial="hidden"
                    animate="show"
                    exit="exit"
                    className="flex min-h-0 min-w-0 flex-1 flex-col"
                  >
                    {rightTab === "preview"
                      ? <PreviewPane ws={workspace} compact={false} />
                      : <FilesPane ws={workspace} compact={false} />}
                  </motion.div>
                </AnimatePresence>
          </ResizablePanel>
        </ResizablePanelGroup>
      ) : (
        <>
          <StackedTabs tab={stacked} base={base} compact={!isDesktop} />
          <div className="flex min-h-0 min-w-0 flex-1 flex-col">
            {pane === "transcript" && <Transcript agent={agent} ws={workspace} />}
            {pane === "files" && <FilesPane ws={workspace} compact={!isDesktop} />}
            {pane === "preview" && <PreviewPane ws={workspace} compact={!isDesktop} />}
          </div>
        </>
      )}

      <SessionOverlays name={agent.name} sessions={agent.sessions} />
    </div>
  )
}

export function SessionScreen({ pane }: { pane: SessionPane }) {
  const { ws, id } = useParams()
  const navigate = useNavigate()
  const workspace = workspaceById(ws ?? "project-jag")
  const agent = agents.find((a) => a.id === id) ?? agentById(workspace.agents[0].id)
  const base = `/w/${workspace.id}/agent/${agent.id}`
  const wide = useIsWide()

  return (
    <SessionControlsProvider key={agent.id} agentModel={agent.model}>
      {/* A pin sends its note to the composer. On a split the transcript is already beside
          the preview; when the panes are stacked it has to come forward. */}
      <ComposeProvider onSend={() => { if (!wide) navigate(base) }}>
        <SessionBody pane={pane} base={base} />
      </ComposeProvider>
    </SessionControlsProvider>
  )
}
