// The delivery ledger. Decision 19 says mail is addressed, delivered as the agent's next
// turn, and acked by delivery id. That is the whole point of this screen, so the counts
// sit under the thread header and the ledger closes the thread instead of leaving a void.
import { Check, Clock } from "lucide-react"
import type { Message } from "@/data"
import type { Thread } from "@/screens/messages/threads"

export type Ledger = { total: number; acked: number; waiting: number }

export function ledgerOf(rows: Message[]): Ledger {
  const acked = rows.filter((m) => m.acked).length
  return { total: rows.length, acked, waiting: rows.length - acked }
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`

// One quiet line under the thread header: the address and where the mail stands.
export function DeliveryStrip({ t, ledger }: { t: Thread; ledger: Ledger }) {
  return (
    <div className="text-muted-foreground flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
      <span className="flex items-center gap-1.5">
        address
        <span className="text-foreground font-mono">{t.id}</span>
      </span>
      <span className="tnum">{plural(ledger.total, "message", "messages")}</span>
      {ledger.acked > 0 && (
        <span className="text-ok tnum flex items-center gap-1">
          <Check className="size-3.5" />
          {ledger.acked} acked
        </span>
      )}
      {ledger.waiting > 0 && (
        <span className="text-warn tnum flex items-center gap-1">
          <Clock className="size-3.5" />
          {ledger.waiting} waiting
        </span>
      )}
    </div>
  )
}

// The end of the thread. It says what an ack means and what the next id will be, so a
// short thread ends on a fact rather than on empty space. The counts live in the strip
// under the header; this block does not repeat them.
export function DeliveryLedger({ t, ledger }: { t: Thread; ledger: Ledger }) {
  const who = t.kind === "channel" ? `every agent in ${t.label.slice(1)}` : t.label
  return (
    <div className="border-border bg-canvas max-w-prose rounded-xl border p-4">
      <p className="u-overline text-muted-foreground">End of thread</p>
      <p className="mt-2 text-sm leading-relaxed">
        A message is acked when the turn that carried it ends.{" "}
        {ledger.waiting === 0
          ? `Everything here is acked, so ${who} has read all of it.`
          : `${plural(ledger.waiting, "message is", "messages are")} still waiting, so ${who} has not taken a turn since ${ledger.waiting === 1 ? "it" : "they"} arrived.`}
      </p>
      <p className="text-muted-foreground mt-2 text-sm leading-relaxed">
        Everything here is addressed to <span className="text-foreground font-mono">{t.id}</span> and carries its own
        delivery id, so an ack never has to guess which message it answers.
      </p>
    </div>
  )
}
