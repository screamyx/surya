// Screen: /w/:ws/messages - feature 9, agent mail. Decision 19: mail is addressed, delivered as
// the agent's next turn, and acked by delivery id. Desktop is list beside thread, phone is one
// at a time.
import { useMemo, useState } from "react"
import { useParams } from "react-router"
import { cn } from "@/lib/utils"
import { workspaceById, workspaces, type Message } from "@/data"
import { ThreadList } from "@/screens/messages/thread-list"
import { ThreadView } from "@/screens/messages/thread"
import { deliveryFor, threadsFor } from "@/screens/messages/threads"

// The mockup's "now". Anything you send in this screen lands at this minute.
const now = "2026-09-05T01:14:00+08:00"

export function MessagesScreen() {
  const { ws } = useParams()
  const w = workspaces.some((x) => x.id === ws) ? workspaceById(ws!) : workspaces[0]
  const threads = useMemo(() => threadsFor(w), [w])

  const [selected, setSelected] = useState(threads[0].id)
  const [sent, setSent] = useState<Message[]>([])
  const [openOnPhone, setOpenOnPhone] = useState(false)

  const thread = threads.find((t) => t.id === selected) ?? threads[0]

  const open = (id: string) => {
    setSelected(id)
    setOpenOnPhone(true)
  }

  const send = (text: string) => {
    setSent((prev) => [
      ...prev,
      { id: `m-you-${prev.length}`, from: "you", to: thread.id, text, at: now, deliveryId: deliveryFor(thread.id), acked: false },
    ])
  }

  const mine = sent.filter((m) => m.to === thread.id)

  return (
    <div className="flex h-[calc(100svh-3rem-2.75rem-5rem)] flex-col md:h-[calc(100svh-3rem-2.75rem)]">
      <div className="shrink-0 border-b px-4 py-3">
        <h1 className="font-heading text-xl font-semibold tracking-tight">Messages</h1>
        <p className="text-muted-foreground text-sm">
          Mail between you and your agents, and between agents. Every message has an address, a delivery id, and an ack.
        </p>
      </div>

      <div className="flex min-h-0 flex-1">
        <div className={cn("min-h-0 w-full shrink-0 overflow-y-auto md:w-70 md:border-r", openOnPhone && "max-md:hidden")}>
          <ThreadList threads={threads} selected={selected} onSelect={open} />
        </div>
        <div className={cn("min-h-0 min-w-0 flex-1 flex-col", openOnPhone ? "flex" : "hidden md:flex")}>
          <ThreadView t={thread} sent={mine} onSend={send} onBack={() => setOpenOnPhone(false)} />
        </div>
      </div>
    </div>
  )
}
