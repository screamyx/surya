// The two shapes an ask takes on Home: the first one open at full weight, and the
// rest condensed to a line. Stacked correspondence, not a grid of equal cards.
// Answering the lead does not decide anything inside this file: it reports the answer
// up to Home, which is what reorders the queue. That way the movement and the truth
// are the same event.
import { Link } from "react-router"
import { ArrowRight, Check, ExternalLink, MapPin, Rocket, RotateCcw, X } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { agentById, fmtTime, workspaceById, type InboxItem } from "@/data"
import { cn } from "@/lib/utils"

export type Answer = "approved" | "rejected"

const kindWord: Record<InboxItem["kind"], string> = {
  permission: "wants approval",
  question: "has a question",
  failed: "stopped",
  pin: "picked up your pin",
  result: "finished",
}

const kindIcon = { permission: Check, question: ArrowRight, failed: RotateCcw, pin: MapPin, result: Rocket }

// What the condensed row says once the ask has been answered. The word has to match the
// button that was pressed: a permission is approved, everything else is opened.
const answerWord = (item: InboxItem, a: Answer) =>
  a === "rejected" ? "Rejected" : item.kind === "permission" ? "Approved" : "Opened"

function Who({ item }: { item: InboxItem }) {
  const agent = agentById(item.agentId)
  return (
    <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-sm">
      <span className="text-foreground font-medium">{agent.name}</span>
      <span className="text-muted-foreground">{kindWord[item.kind]} in</span>
      <span className="text-muted-foreground font-mono text-xs">{workspaceById(item.workspaceId).name}</span>
    </span>
  )
}

// The one at the top of the queue. It is open, it shows the command, and it can be
// answered here without going anywhere.
export function LeadAsk({ item, onAnswer }: { item: InboxItem; onAnswer: (a: Answer) => void }) {
  const Icon = kindIcon[item.kind]
  return (
    <article className="bg-card shadow-lift rounded-xl">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 px-5 pt-5">
        <Who item={item} />
        <span className="text-muted-foreground tnum ml-auto text-xs">{fmtTime(item.at)}</span>
      </div>
      <h3 className="px-5 pt-2 text-xl leading-snug font-medium">{item.title}</h3>
      {item.command && (
        <div className="bg-canvas mx-5 mt-3 overflow-hidden rounded-lg">
          <div className="text-muted-foreground border-b px-3 py-1.5 font-mono text-xs">{item.tool}</div>
          <pre className="overflow-x-auto px-3 py-2 font-mono text-xs break-words whitespace-pre-wrap">{item.command}</pre>
        </div>
      )}
      <p className="text-muted-foreground px-5 pt-3 text-sm">{item.body}</p>
      <div className="mt-4 flex flex-col gap-2 border-t px-5 py-4 sm:flex-row sm:items-center">
        <Button className="h-11 w-full sm:h-8 sm:w-auto" onClick={() => onAnswer("approved")}>
          <Icon data-icon="inline-start" />
          {item.kind === "permission" ? "Approve" : "Open it"}
        </Button>
        <Button variant="outline" className="h-11 w-full sm:h-8 sm:w-auto" onClick={() => onAnswer("rejected")}>
          <X data-icon="inline-start" />
          Reject
        </Button>
        <Button
          variant="ghost"
          size="sm"
          nativeButton={false}
          className="text-muted-foreground sm:ml-auto"
          render={<Link to={`/w/${item.workspaceId}/agent/${item.agentId}`} />}
        >
          Open agent
          <ExternalLink data-icon="inline-end" />
        </Button>
      </div>
    </article>
  )
}

// Everything behind the first one, and everything already answered. A line, not a card.
// An answered row carries the word it was answered with and a way back, so the queue
// never swallows something without saying where it went.
export function AskRow({ item, answer, onUndo }: { item: InboxItem; answer?: Answer; onUndo?: () => void }) {
  if (answer && onUndo) {
    return (
      <div className="flex min-w-0 items-center gap-3 rounded-lg px-3 py-2.5">
        <span className={cn("size-2 shrink-0 rounded-full", answer === "approved" ? "bg-ok" : "bg-border-strong")} />
        <span className="min-w-0 flex-1">
          <span className="text-muted-foreground block truncate text-sm line-through">{item.title}</span>
          <Who item={item} />
        </span>
        {/* Secondary, not the accent: an answered line is history, and the accent on this
            screen belongs to the ask still waiting. */}
        <Badge variant="secondary" className="shrink-0">{answerWord(item, answer)}</Badge>
        <Button variant="ghost" size="sm" className="text-muted-foreground shrink-0" onClick={onUndo}>
          Undo
        </Button>
      </div>
    )
  }
  return (
    <Link
      to="/inbox"
      className="hover:bg-accent group flex min-w-0 items-center gap-3 rounded-lg px-3 py-2.5 transition-colors"
    >
      <span className={cn("size-2 shrink-0 rounded-full", item.kind === "failed" ? "bg-destructive" : "bg-primary")} />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium">{item.title}</span>
        <Who item={item} />
      </span>
      <span className="text-muted-foreground tnum hidden shrink-0 text-xs sm:inline">{fmtTime(item.at)}</span>
      <ArrowRight className="text-muted-foreground size-4 shrink-0 opacity-0 transition-opacity group-hover:opacity-100" />
    </Link>
  )
}
