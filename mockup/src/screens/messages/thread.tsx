// Right column: one thread, its messages, and the box that sends the next one.
import { useState } from "react"
import { motion } from "motion/react"
import { ArrowLeft, Hash, Send } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty"
import { Textarea } from "@/components/ui/textarea"
import { StatusDot } from "@/components/status"
import type { Message } from "@/data"
import { rise, stagger } from "@/motion"
import { CrossBlock, MessageRow } from "@/screens/messages/parts"
import { byTime, channelMail, crossMail, deliveryFor, directMail, type Thread } from "@/screens/messages/threads"

function ThreadHeader({ t, onBack }: { t: Thread; onBack?: () => void }) {
  return (
    <div className="flex items-center gap-2.5 border-b px-4 py-3">
      {onBack && (
        <Button size="icon-sm" variant="ghost" onClick={onBack} aria-label="Back to all threads" className="-ml-1 shrink-0 lg:hidden">
          <ArrowLeft />
        </Button>
      )}
      {t.kind === "channel" ? <Hash className="text-muted-foreground size-4 shrink-0" /> : <StatusDot status={t.agent.status} />}
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <h2 className="font-heading truncate text-base font-medium">{t.label}</h2>
          {t.kind === "direct" && <Badge variant="outline" className="font-mono text-xs">{t.agent.model}</Badge>}
        </div>
        <p className="text-muted-foreground truncate text-xs">
          {t.kind === "channel" ? "Every agent in this workspace gets it" : t.agent.summary}
        </p>
      </div>
    </div>
  )
}

function Composer({ t, onSend }: { t: Thread; onSend: (text: string) => void }) {
  const [text, setText] = useState("")
  const lands = t.kind === "channel"
    ? `Lands as the next turn for every agent in ${t.label.slice(1)}. No polling.`
    : `Lands as ${t.label}'s next turn. No polling.`
  const send = () => {
    if (!text.trim()) return
    onSend(text.trim())
    setText("")
  }
  return (
    <div className="bg-background space-y-2 border-t px-4 py-3">
      <Textarea
        name="compose"
        rows={2}
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder={`Message ${t.label}`}
        className="min-h-16 resize-none"
      />
      <div className="flex flex-wrap items-center gap-2">
        <p className="text-muted-foreground min-w-0 flex-1 text-xs">{lands}</p>
        <Button size="sm" onClick={send} disabled={!text.trim()} className="shrink-0 max-sm:h-10">
          <Send data-icon="inline-start" />
          Send
        </Button>
      </div>
    </div>
  )
}

export function ThreadView({ t, sent, onSend, onBack }: {
  t: Thread
  sent: Message[]
  onSend: (text: string) => void
  onBack?: () => void
}) {
  const base = t.kind === "channel" ? channelMail(t.id) : directMail(t.id)
  const rows = byTime([...base, ...sent])
  const cross = t.kind === "direct" ? crossMail(t.id) : []

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <ThreadHeader t={t} onBack={onBack} />
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-5">
        <motion.div key={t.id} variants={stagger} initial="hidden" animate="show" className="mx-auto mt-auto flex w-full max-w-3xl flex-col gap-5">
          {rows.map((m) => (
            <motion.div key={m.id} variants={rise}>
              <MessageRow m={m} />
            </motion.div>
          ))}
          {rows.length === 0 && (
            <Empty className="py-8">
              <EmptyHeader>
                <EmptyTitle>No mail with {t.label} yet</EmptyTitle>
                <EmptyDescription>
                  Anything you send here queues on {t.label}'s address, id {deliveryFor(t.id)} next.
                </EmptyDescription>
              </EmptyHeader>
            </Empty>
          )}
          {cross.length > 0 && (
            <motion.div variants={rise}>
              <CrossBlock id={t.id} groups={cross} />
            </motion.div>
          )}
        </motion.div>
      </div>
      <Composer t={t} onSend={onSend} />
    </div>
  )
}
