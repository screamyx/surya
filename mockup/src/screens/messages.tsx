// Screen: /w/:ws/messages - feature 9, agent mail. Decision 19: mail is addressed,
// delivered as the agent's next turn, and acked by delivery id.
// Desktop is the thread list beside the thread. Phone shows one at a time: the list,
// then the thread with a back button, and the page title gets out of the way so a
// thread has the whole pane (decision 20, point 4: the phone cuts rather than shrinks).
import { useMemo, useState } from "react"
import { useParams } from "react-router"
import { cn } from "@/lib/utils"
import { workspaceById, workspaces, type Message } from "@/data"
import { ThreadList } from "@/screens/messages/thread-list"
import { ThreadView } from "@/screens/messages/thread"
import { deliveryFor, threadsFor, type Thread } from "@/screens/messages/threads"
import { BeatCount } from "@/screens/beat"

// The mockup's "now". Anything you send in this screen lands at this minute.
const now = "2026-09-05T01:14:00+08:00"

export function MessagesScreen() {
  const { ws } = useParams()
  const w = workspaces.some((x) => x.id === ws) ? workspaceById(ws!) : workspaces[0]
  const threads = useMemo(() => threadsFor(w), [w])

  const [selected, setSelected] = useState(threads[0].id)
  const [sent, setSent] = useState<Message[]>([])
  const [openOnPhone, setOpenOnPhone] = useState(false)
  // Opening a thread reads it. The badge and the waiting count both fall by that much,
  // so the number that beats is a number that actually changed.
  const [read, setRead] = useState<string[]>([])

  const unreadOf = (t: Thread) => (read.includes(t.id) ? 0 : t.unread)
  const thread = threads.find((t) => t.id === selected) ?? threads[0]
  const waiting = threads.reduce((n, t) => n + unreadOf(t), 0)

  const open = (id: string) => {
    setSelected(id)
    setOpenOnPhone(true)
    setRead((prev) => (prev.includes(id) ? prev : [...prev, id]))
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
      <div className={cn("shrink-0 border-b px-4 py-3", openOnPhone && "max-lg:hidden")}>
        <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <h1 className="u-display text-3xl md:text-4xl">Messages</h1>
          <p className="text-muted-foreground tnum text-sm">
            {threads.length} threads in {w.name}
            {waiting > 0 && <>, <BeatCount value={waiting} /> waiting for you</>}
          </p>
        </div>
        <p className="text-muted-foreground mt-1 max-w-prose text-sm">
          Every message has an address, a delivery id, and an ack.
        </p>
      </div>

      <div className="flex min-h-0 flex-1">
        <div className={cn("min-h-0 w-full shrink-0 overflow-y-auto lg:w-70 lg:border-r", openOnPhone && "max-lg:hidden")}>
          <ThreadList threads={threads} selected={selected} onSelect={open} unreadOf={unreadOf} />
        </div>
        <div className={cn("min-h-0 min-w-0 flex-1 flex-col", openOnPhone ? "flex" : "hidden lg:flex")}>
          <ThreadView t={thread} sent={mine} onSend={send} onBack={() => setOpenOnPhone(false)} />
        </div>
      </div>
    </div>
  )
}
