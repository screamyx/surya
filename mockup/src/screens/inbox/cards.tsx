// One card per inbox kind. Every card carries the same header, the same body width,
// and its own row of actions, so the list reads as one thing on a phone.
//
// A card does not keep its own answer. It reports one up with onResolve, and the stack
// collapses it to an answered line. That keeps one truth about whether an ask is done,
// and it is the same event that drives the movement.
import { useState } from "react"
import { Link } from "react-router"
import { AnimatePresence, motion } from "motion/react"
import { Check, ExternalLink, MapPin, Rocket, RotateCcw, X } from "lucide-react"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Item, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item"
import { Textarea } from "@/components/ui/textarea"
import { StatusBadge } from "@/components/status"
import { AlwaysAllow, RuleMade } from "@/screens/inbox/always-allow"
import type { CreatedRule } from "@/screens/inbox/rule-draft"
import { agentById, fmtTime, workspaceById, type InboxItem } from "@/data"
import { pop } from "@/motion"
import { cn } from "@/lib/utils"

// What the answered line says, per card. Plain words, past tense, no invented state.
export type Resolve = (outcome: string) => void

// Phone: thumb sized and full width. Desktop: a normal button row.
const thumb = "h-11 w-full sm:h-8 sm:w-auto"

function CardTop({ item, trailing }: { item: InboxItem; trailing?: React.ReactNode }) {
  const agent = agentById(item.agentId)
  const ws = workspaceById(item.workspaceId)
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Avatar className="size-7">
        <AvatarFallback className="text-xs uppercase">{agent.name.slice(0, 1)}</AvatarFallback>
      </Avatar>
      <span className="font-medium">{agent.name}</span>
      <Badge variant="outline">{ws.name}</Badge>
      {trailing}
      <span className="text-muted-foreground ml-auto text-xs tabular-nums">{fmtTime(item.at)}</span>
    </div>
  )
}

function OpenAgent({ item, variant = "ghost" }: { item: InboxItem; variant?: "ghost" | "outline" }) {
  return (
    <Button variant={variant} size="sm" nativeButton={false} className={cn(variant === "ghost" && "text-muted-foreground")} render={<Link to={`/w/${item.workspaceId}/agent/${item.agentId}`} />}>
      Open agent
      <ExternalLink data-icon="inline-end" />
    </Button>
  )
}

function PermissionCard({ item, onResolve }: { item: InboxItem; onResolve: Resolve }) {
  // Approve and Reject answer the ask, so the stack collapses the card. Always allow
  // keeps a rule as well, and that has its own undo, so it stays here and pops in.
  const [rule, setRule] = useState<CreatedRule | null>(null)
  return (
    <Card>
      <CardHeader>
        <CardTop item={item} />
        <CardTitle className="mt-2">{item.title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="bg-muted overflow-hidden rounded-lg">
          <div className="text-muted-foreground border-b px-3 py-1.5 font-mono text-xs">{item.tool}</div>
          <pre className="px-3 py-2 font-mono text-xs whitespace-pre-wrap break-words">{item.command}</pre>
        </div>
        <p className="text-muted-foreground">{item.body}</p>
      </CardContent>
      <CardFooter className="flex-col items-stretch gap-2 sm:flex-row sm:items-center">
        <AnimatePresence mode="wait" initial={false}>
          {rule ? (
            <motion.div key="rule" variants={pop} initial="hidden" animate="show" exit="hidden" className="w-full">
              <RuleMade rule={rule} onUndo={() => setRule(null)} />
            </motion.div>
          ) : (
            <motion.div
              key="ask"
              variants={pop}
              initial="hidden"
              animate="show"
              exit="hidden"
              className="flex w-full flex-col items-stretch gap-2 sm:flex-row sm:items-center"
            >
              <Button className={thumb} onClick={() => onResolve("Approved")}>
                <Check data-icon="inline-start" />
                Approve
              </Button>
              <Button variant="outline" className={thumb} onClick={() => onResolve("Rejected")}>
                <X data-icon="inline-start" />
                Reject
              </Button>
              <div className="flex flex-col items-stretch gap-1 sm:ml-auto sm:flex-row sm:items-center">
                <AlwaysAllow item={item} onCreate={setRule} />
                <OpenAgent item={item} />
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </CardFooter>
    </Card>
  )
}

function QuestionCard({ item, onResolve }: { item: InboxItem; onResolve: Resolve }) {
  const [picked, setPicked] = useState<string | null>(null)
  const [other, setOther] = useState("")
  const answer = other.trim() || picked
  return (
    <Card>
      <CardHeader>
        <CardTop item={item} />
        <CardTitle className="mt-2">{item.title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <p className="text-muted-foreground">{item.body}</p>
        <div className="flex flex-col gap-2">
          {item.options?.map((o) => (
            <Item
              key={o.label}
              variant="outline"
              className={cn("cursor-pointer text-left", picked === o.label && "border-primary bg-muted")}
              render={<button type="button" onClick={() => setPicked(o.label)} />}
            >
              <ItemContent>
                <ItemTitle className="font-semibold">{o.label}</ItemTitle>
                <ItemDescription>{o.description}</ItemDescription>
              </ItemContent>
              <AnimatePresence initial={false}>
                {picked === o.label && (
                  <motion.span key="mark" variants={pop} initial="hidden" animate="show" exit="hidden" className="inline-flex">
                    <Check className="text-primary size-4" />
                  </motion.span>
                )}
              </AnimatePresence>
            </Item>
          ))}
        </div>
        <Textarea
          value={other}
          onChange={(e) => setOther(e.target.value)}
          placeholder="Other - type your own answer"
          className="min-h-20"
        />
      </CardContent>
      <CardFooter className="flex-col items-stretch gap-2 sm:flex-row sm:items-center">
        <Button className={thumb} disabled={!answer} onClick={() => answer && onResolve(`Answered: ${answer}`)}>
          Send answer
        </Button>
        <div className="sm:ml-auto">
          <OpenAgent item={item} />
        </div>
      </CardFooter>
    </Card>
  )
}

function PinCard({ item, onResolve }: { item: InboxItem; onResolve: Resolve }) {
  return (
    <Card>
      <CardHeader>
        <CardTop item={item} />
        <CardTitle className="mt-2">{item.title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-3 sm:flex-row sm:items-start sm:gap-4">
        <div className="bg-muted relative aspect-video w-full max-w-40 shrink-0 rounded-md border">
          <span className="bg-destructive text-destructive-foreground absolute top-1/3 left-1/2 flex size-6 -translate-x-1/2 items-center justify-center rounded-full">
            <MapPin className="size-3.5" />
          </span>
        </div>
        <p className="text-muted-foreground">{item.body}</p>
      </CardContent>
      <CardFooter className="flex-col items-stretch gap-2 sm:flex-row sm:items-center">
        <Button className={thumb} onClick={() => onResolve("Accepted")}>
          Accept
        </Button>
        <Button variant="outline" className={thumb} onClick={() => onResolve("Replied")}>
          Reply
        </Button>
        <div className="sm:ml-auto">
          <OpenAgent item={item} />
        </div>
      </CardFooter>
    </Card>
  )
}

function ResultCard({ item, onResolve }: { item: InboxItem; onResolve: Resolve }) {
  return (
    <Card>
      <CardHeader>
        <CardTop item={item} />
        <CardTitle className="mt-2">{item.title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <p className="text-muted-foreground">{item.body}</p>
        <div className="flex flex-wrap gap-2">
          <Badge variant="secondary">PR #648</Badge>{/* ds-allow-hardcode: a pull-request number, not a colour */}
          <Badge variant="secondary">CI green</Badge>
        </div>
      </CardContent>
      <CardFooter className="flex-col items-stretch gap-2 sm:flex-row sm:items-center">
        <Button className={thumb} onClick={() => onResolve("Shipping")}>
          <Rocket data-icon="inline-start" />
          Ship
        </Button>
        <Button variant="outline" className={thumb} nativeButton={false} render={<Link to={`/w/${item.workspaceId}/result/${item.agentId}`} />}>
          See result
        </Button>
        <div className="sm:ml-auto">
          <OpenAgent item={item} />
        </div>
      </CardFooter>
    </Card>
  )
}

// A stopped agent wants Retry or Give up, not an answer. The reason leads, the detail
// says how far it got, so you can decide without opening the feed.
function FailedCard({ item, onResolve }: { item: InboxItem; onResolve: Resolve }) {
  const agent = agentById(item.agentId)
  const failure = agent.failure
  return (
    <Card className="ring-destructive/30 ring-2">
      <CardHeader>
        <CardTop item={item} trailing={<StatusBadge status="failed" />} />
        <CardTitle className="mt-2">{item.title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        {failure && <p className="font-semibold">{failure.reason}</p>}
        <p className="text-muted-foreground">{failure?.detail ?? item.body}</p>
      </CardContent>
      <CardFooter className="flex-col items-stretch gap-2 sm:flex-row sm:items-center">
        {failure?.retryable !== false && (
          <Button className={thumb} onClick={() => onResolve("Retrying")}>
            <RotateCcw data-icon="inline-start" />
            Retry
          </Button>
        )}
        <Button variant="outline" className={thumb} nativeButton={false} render={<Link to={`/w/${item.workspaceId}/agent/${item.agentId}`} />}>
          Open agent
        </Button>
        <Button
          variant="ghost"
          className={cn(thumb, "text-destructive hover:text-destructive")}
          onClick={() => onResolve("Given up")}
        >
          Give up
        </Button>
      </CardFooter>
    </Card>
  )
}

export function InboxCard({ item, onResolve }: { item: InboxItem; onResolve: Resolve }) {
  if (item.kind === "permission") return <PermissionCard item={item} onResolve={onResolve} />
  if (item.kind === "question") return <QuestionCard item={item} onResolve={onResolve} />
  if (item.kind === "pin") return <PinCard item={item} onResolve={onResolve} />
  if (item.kind === "failed") return <FailedCard item={item} onResolve={onResolve} />
  return <ResultCard item={item} onResolve={onResolve} />
}
