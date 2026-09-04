// The bridge between the panes. The preview drops a pin into the composer, so picking
// something in the running app becomes the agent's next instruction without leaving the
// screen. Windsurf calls this "Send element"; it is the reason the panes share a surface.
import { createContext, use, useState } from "react"

type Compose = {
  draft: string
  setDraft: (text: string) => void
  // True right after another pane wrote the draft, so the composer knows to take focus.
  handed: boolean
  taken: () => void
  // Fill the composer and put the transcript in front of the person, phone included.
  send: (text: string) => void
}

const ComposeContext = createContext<Compose | undefined>(undefined)

export function ComposeProvider({ onSend, children }: { onSend: () => void; children: React.ReactNode }) {
  const [draft, setDraft] = useState("")
  const [handed, setHanded] = useState(false)

  const value: Compose = {
    draft,
    setDraft: (text) => { setDraft(text); setHanded(false) },
    handed,
    taken: () => setHanded(false),
    send: (text) => { setDraft(text); setHanded(true); onSend() },
  }
  return <ComposeContext value={value}>{children}</ComposeContext>
}

export function useCompose() {
  const ctx = use(ComposeContext)
  if (!ctx) throw new Error("useCompose must be used inside the session surface")
  return ctx
}
