// The pieces a thread is made of: one message row, the delivery mark under it,
// and the muted block of mail an agent exchanged with other agents.
import { ArrowLeftRight, ArrowRight, Check, ChevronRight } from "lucide-react"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { Badge } from "@/components/ui/badge"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import { fmtTime, type Message } from "@/data"
import { isChannel } from "@/screens/messages/threads"
import { cn } from "@/lib/utils"

// Every message carries an address, a delivery id, and an ack. This is the ack.
export function MailMeta({ m, className }: { m: Message; className?: string }) {
  return (
    <span className={cn("flex flex-wrap items-center gap-2", className)}>
      <span className="text-muted-foreground font-mono text-xs">{m.deliveryId}</span>
      {m.acked ? (
        <span className="text-muted-foreground flex items-center gap-1 text-xs">
          <Check className="size-3" />
          acked
        </span>
      ) : (
        <Badge variant="outline" className="h-4 px-1.5 text-[10px]">unacked</Badge>
      )}
    </span>
  )
}

export function MessageRow({ m }: { m: Message }) {
  if (m.from === "you") {
    return (
      <div className="flex flex-col items-end gap-1">
        <div className="bg-primary text-primary-foreground max-w-[85%] rounded-2xl px-4 py-2.5 text-sm leading-relaxed">
          {m.text}
        </div>
        <span className="flex items-center gap-2">
          <span className="text-muted-foreground text-xs tabular-nums">{fmtTime(m.at)}</span>
          <MailMeta m={m} />
        </span>
      </div>
    )
  }
  return (
    <div className="flex gap-3">
      <Avatar className="size-6 shrink-0">
        <AvatarFallback className="text-xs uppercase">{m.from.slice(0, 1)}</AvatarFallback>
      </Avatar>
      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-sm font-medium">{m.from}</span>
          {isChannel(m.to) && <Badge variant="outline" className="font-mono text-[10px]">{m.to}</Badge>}
          <span className="text-muted-foreground text-xs tabular-nums">{fmtTime(m.at)}</span>
        </div>
        <p className="text-sm leading-relaxed">{m.text}</p>
        <MailMeta m={m} />
      </div>
    </div>
  )
}

function CrossRow({ m }: { m: Message }) {
  return (
    <div className="space-y-1 py-2">
      <div className="text-muted-foreground flex flex-wrap items-center gap-1.5 text-xs">
        <span className="font-medium">{m.from}</span>
        <ArrowRight className="size-3" />
        <span className="font-medium">{m.to}</span>
        <span className="tabular-nums">{fmtTime(m.at)}</span>
      </div>
      <p className="text-muted-foreground text-sm leading-relaxed">{m.text}</p>
      <MailMeta m={m} />
    </div>
  )
}

// Agent-to-agent traffic on this agent, folded away so it never crowds your own thread.
export function CrossBlock({ id, groups }: { id: string; groups: { other: string; rows: Message[] }[] }) {
  if (!groups.length) return null
  return (
    <div className="space-y-2 rounded-lg border border-dashed p-3">
      <p className="text-muted-foreground text-xs">Mail between {id} and other agents</p>
      {groups.map((g) => (
        <Collapsible key={g.other}>
          <CollapsibleTrigger className="text-muted-foreground hover:text-foreground group flex w-full items-center gap-1.5 text-xs transition-colors">
            <ChevronRight className="size-3 transition-transform group-data-[panel-open]:rotate-90" />
            <span className="font-medium">{id}</span>
            <ArrowLeftRight className="size-3" />
            <span className="font-medium">{g.other}</span>
            <span>{g.rows.length === 1 ? "1 message" : `${g.rows.length} messages`}</span>
          </CollapsibleTrigger>
          <CollapsibleContent>
            <div className="divide-y border-l pl-3">
              {g.rows.map((m) => <CrossRow key={m.id} m={m} />)}
            </div>
          </CollapsibleContent>
        </Collapsible>
      ))}
    </div>
  )
}
