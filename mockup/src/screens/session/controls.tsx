// Session state that more than one pane needs: which model is running, whether you stopped
// the agent, which sheet is open, and the lines surya wrote into the transcript itself.
// It lives in the shell so the header keeps working while the phone shows the files pane.
import { createContext, use, useState } from "react"
import { Sparkles, StopCircle } from "lucide-react"
import type { SystemRow } from "@/screens/session/system-line"
import { modelName, type Effort } from "@/screens/session/model-sheet"
import { fmtTime } from "@/data"

// The mockup's "now". Anything you do in this screen lands at this minute.
const now = "2026-09-05T01:14:00+08:00"

type Controls = {
  model: string
  effort: Effort
  stopped: boolean
  rows: SystemRow[]
  modelOpen: boolean
  sessionsOpen: boolean
  stopOpen: boolean
  setModelOpen: (v: boolean) => void
  setSessionsOpen: (v: boolean) => void
  setStopOpen: (v: boolean) => void
  switchModel: (model: string, effort: Effort) => void
  stop: () => void
  resume: () => void
}

const ControlsContext = createContext<Controls | undefined>(undefined)

export function SessionControlsProvider({ agentModel, children }: { agentModel: string; children: React.ReactNode }) {
  const [model, setModel] = useState(agentModel)
  const [effort, setEffort] = useState<Effort>("high")
  const [stopped, setStopped] = useState(false)
  const [rows, setRows] = useState<SystemRow[]>([])
  const [modelOpen, setModelOpen] = useState(false)
  const [sessionsOpen, setSessionsOpen] = useState(false)
  const [stopOpen, setStopOpen] = useState(false)

  const add = (row: SystemRow) => setRows((r) => [...r, row])

  // The wording is what Claude Code itself replies to "/model <id>", probed 2026-09-05.
  const switchModel = (next: string, level: Effort) => {
    setModel(next)
    setEffort(level)
    setModelOpen(false)
    add({ id: `sys-model-${rows.length}`, text: `Set model to ${modelName(next)} for this session only`, at: now, Icon: Sparkles })
  }

  const stop = () => {
    setStopped(true)
    setStopOpen(false)
    add({ id: `sys-stop-${rows.length}`, text: `Stopped by you at ${fmtTime(now)} · the running Bash was interrupted`, Icon: StopCircle })
  }

  const value: Controls = {
    model, effort, stopped, rows, modelOpen, sessionsOpen, stopOpen,
    setModelOpen, setSessionsOpen, setStopOpen, switchModel, stop,
    resume: () => setStopped(false),
  }
  return <ControlsContext value={value}>{children}</ControlsContext>
}

export function useSessionControls() {
  const ctx = use(ControlsContext)
  if (!ctx) throw new Error("useSessionControls must be used inside the session surface")
  return ctx
}
