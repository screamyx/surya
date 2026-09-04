// Right column: one thread, its messages, and the box that sends the next one.
// The thread reads from the top. It used to be bottom-aligned in a tall pane, which put
// 300 pixels of nothing between the header and the first message; a short thread now
// starts where a reader looks and closes on the delivery ledger. A thread with no mail
// gets a real empty state that owns the pane instead.
import { useState } from "react"
import { AnimatePresence, motion } from "motion/react"
import { ArrowLeft, Hash, Send } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty"
import { Textarea } from "@/components/ui/textarea"
import { StatusDot } from "@/components/status"
import type { Message } from "@/data"
import { layoutSpring, pop, rise, stagger, swap } from "@/motion"
import { DeliveryLedger, DeliveryStrip, ledgerOf } from "@/screens/messages/delivery"
import { CrossBlock, MessageRow } from "@/screens/messages/parts"
import { byTime, channelMail, crossMail, directMail, type Thread } from "@/screens/messages/threads"

function ThreadHeader({ t, rows, onBack }: { t: Thread; rows: Message[]; onBack?: () => void }) {
  return (
    <div className="bg-background shrink-0 border-b px-4 py-3">
      <div className="flex items-center gap-2.5">
        {onBack && (
          <Button
            size="icon-sm"
            variant="outline"
            onClick={onBack}
            aria-label="Back to all threads"
            className="-ml-1 size-9 shrink-0 lg:hidden"
          >
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
      <DeliveryStrip t={t} ledger={ledgerOf(rows)} />
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
    <div className="bg-background shrink-0 space-y-2 border-t px-4 py-3">
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
        <Button size="sm" onClick={send} disabled={!text.trim()} className="shrink-0 max-sm:h-11">
          <Send data-icon="inline-start" />
          Send
        </Button>
      </div>
    </div>
  )
}

function StartMark({ label }: { label: string }) {
  return (
    <div className="flex items-center gap-3">
      <div className="u-seam flex-1" />
      <span className="u-overline text-muted-foreground shrink-0">{label}</span>
      <div className="u-seam flex-1" />
    </div>
  )
}

// A thread with no mail owns the pane rather than floating a one-line ledger in it.
function EmptyThread({ t }: { t: Thread }) {
  const who = t.kind === "channel" ? `every agent in ${t.label.slice(1)}` : t.label
  return (
    <Empty className="min-h-full justify-center">
      <EmptyHeader>
        <EmptyTitle>No mail with {t.label} yet</EmptyTitle>
        <EmptyDescription>
          Write below and it goes to {who} as their next turn. It carries a delivery id, and it is acked when the
          turn that carried it ends.
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
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
  const ledger = ledgerOf(rows)
  // A message you just sent is the only row that was not here a second ago, so it is
  // the only one that pops. Everything else arrived with the thread.
  const yours = new Set(sent.map((m) => m.id))

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <ThreadHeader t={t} rows={rows} onBack={onBack} />
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-5">
        {/* One thread leaves before the next arrives, so two threads never overlap. */}
        <AnimatePresence mode="wait" initial={false}>
          <motion.div key={t.id} variants={swap} initial="hidden" animate="show" exit="exit" className="mx-auto flex min-h-full w-full max-w-3xl flex-col">
            <motion.div variants={stagger} initial="hidden" animate="show" className="flex min-h-full flex-col gap-5">
              {rows.length === 0 ? (
                <motion.div variants={rise} className="flex min-h-full flex-1 flex-col">
                  <EmptyThread t={t} />
                </motion.div>
              ) : (
                <>
                  <motion.div variants={rise}>
                    <StartMark label="Start of thread" />
                  </motion.div>
                  {rows.map((m) => (
                    <motion.div
                      key={m.id}
                      layout="position"
                      transition={layoutSpring}
                      variants={yours.has(m.id) ? pop : rise}
                      initial={yours.has(m.id) ? "hidden" : undefined}
                      animate={yours.has(m.id) ? "show" : undefined}
                    >
                      <MessageRow m={m} />
                    </motion.div>
                  ))}
                </>
              )}
              {cross.length > 0 && (
                <motion.div layout="position" transition={layoutSpring} variants={rise}>
                  <CrossBlock id={t.id} groups={cross} />
                </motion.div>
              )}
              {rows.length > 0 && (
                <motion.div layout="position" transition={layoutSpring} variants={rise}>
                  <DeliveryLedger t={t} ledger={ledger} />
                </motion.div>
              )}
            </motion.div>
          </motion.div>
        </AnimatePresence>
      </div>
      <Composer t={t} onSend={onSend} />
    </div>
  )
}
