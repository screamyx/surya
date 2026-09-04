// One card per inbox kind. Every card carries the same header, the same body width,
// and its own row of actions, so the list reads as one thing on a phone.
import { useState } from "react"
import { Link } from "react-router"
import { Check, ExternalLink, MapPin, Rocket, RotateCcw, X } from "lucide-react"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Item, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item"
import { Spinner } from "@/components/ui/spinner"
import { Textarea } from "@/components/ui/textarea"
import { StatusBadge } from "@/components/status"
import { agentById, fmtTime, workspaceById, type InboxItem } from "@/data"
import { cn } from "@/lib/utils"

type Decision = "approved" | "rejected"

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

function PermissionCard({ item }: { item: InboxItem }) {
  const [decided, setDecided] = useState<Decision | null>(null)
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
        {decided ? (
          <Badge variant={decided === "approved" ? "default" : "secondary"} className="self-start">
            {decided === "approved" ? "Approved" : "Rejected"}
          </Badge>
        ) : (
          <>
            <Button className={thumb} onClick={() => setDecided("approved")}>
              <Check data-icon="inline-start" />
              Approve
            </Button>
            <Button variant="outline" className={thumb} onClick={() => setDecided("rejected")}>
              <X data-icon="inline-start" />
              Reject
            </Button>
          </>
        )}
        <div className="sm:ml-auto">
          <OpenAgent item={item} />
        </div>
      </CardFooter>
    </Card>
  )
}

function QuestionCard({ item }: { item: InboxItem }) {
  const [picked, setPicked] = useState<string | null>(null)
  const [other, setOther] = useState("")
  const [sent, setSent] = useState<string | null>(null)
  const answer = other.trim() || picked
  return (
    <Card>
      <CardHeader>
        <CardTop item={item} />
        <CardTitle className="mt-2">{item.title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <p className="text-muted-foreground">{item.body}</p>
        {sent ? (
          <Item variant="muted">
            <ItemContent>
              <ItemTitle>{sent}</ItemTitle>
              <ItemDescription>Sent to {agentById(item.agentId).name}.</ItemDescription>
            </ItemContent>
            <Badge variant="secondary">Answered</Badge>
          </Item>
        ) : (
          <>
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
                  {picked === o.label && <Check className="text-primary size-4" />}
                </Item>
              ))}
            </div>
            <Textarea
              value={other}
              onChange={(e) => setOther(e.target.value)}
              placeholder="Other - type your own answer"
              className="min-h-20"
            />
          </>
        )}
      </CardContent>
      {!sent && (
        <CardFooter className="flex-col items-stretch gap-2 sm:flex-row sm:items-center">
          <Button className={thumb} disabled={!answer} onClick={() => setSent(answer)}>
            Send answer
          </Button>
          <div className="sm:ml-auto">
            <OpenAgent item={item} />
          </div>
        </CardFooter>
      )}
    </Card>
  )
}

function PinCard({ item }: { item: InboxItem }) {
  const [decided, setDecided] = useState<Decision | null>(null)
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
        {decided ? (
          <Badge variant={decided === "approved" ? "default" : "secondary"} className="self-start">
            {decided === "approved" ? "Accepted" : "Replied"}
          </Badge>
        ) : (
          <>
            <Button className={thumb} onClick={() => setDecided("approved")}>
              Accept
            </Button>
            <Button variant="outline" className={thumb} onClick={() => setDecided("rejected")}>
              Reply
            </Button>
          </>
        )}
        <div className="sm:ml-auto">
          <OpenAgent item={item} />
        </div>
      </CardFooter>
    </Card>
  )
}

function ResultCard({ item }: { item: InboxItem }) {
  const [shipped, setShipped] = useState(false)
  return (
    <Card>
      <CardHeader>
        <CardTop item={item} />
        <CardTitle className="mt-2">{item.title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <p className="text-muted-foreground">{item.body}</p>
        <div className="flex flex-wrap gap-2">
          <Badge variant="secondary">PR #648</Badge>
          <Badge variant="secondary">CI green</Badge>
        </div>
      </CardContent>
      <CardFooter className="flex-col items-stretch gap-2 sm:flex-row sm:items-center">
        {shipped ? (
          <Badge className="self-start">Shipping</Badge>
        ) : (
          <Button className={thumb} onClick={() => setShipped(true)}>
            <Rocket data-icon="inline-start" />
            Ship
          </Button>
        )}
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
function FailedCard({ item }: { item: InboxItem }) {
  const agent = agentById(item.agentId)
  const [retrying, setRetrying] = useState(false)
  const failure = agent.failure
  return (
    <Card className="ring-destructive/30 ring-2">
      <CardHeader>
        <CardTop item={item} trailing={retrying ? undefined : <StatusBadge status="failed" />} />
        <CardTitle className="mt-2">{item.title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        {failure && <p className="font-semibold">{failure.reason}</p>}
        <p className="text-muted-foreground">{failure?.detail ?? item.body}</p>
      </CardContent>
      <CardFooter className="flex-col items-stretch gap-2 sm:flex-row sm:items-center">
        {retrying ? (
          <>
            <Badge variant="secondary" className="gap-1.5 self-start">
              <Spinner className="size-3" />
              Retrying
            </Badge>
            <p className="text-muted-foreground text-sm">{agent.name} is back on the task</p>
          </>
        ) : (
          <>
            {failure?.retryable !== false && (
              <Button className={thumb} onClick={() => setRetrying(true)}>
                <RotateCcw data-icon="inline-start" />
                Retry
              </Button>
            )}
            <Button variant="outline" className={thumb} nativeButton={false} render={<Link to={`/w/${item.workspaceId}/agent/${item.agentId}`} />}>
              Open agent
            </Button>
            <Button variant="ghost" className={cn(thumb, "text-destructive hover:text-destructive")}>
              Give up
            </Button>
          </>
        )}
      </CardFooter>
    </Card>
  )
}

export function InboxCard({ item }: { item: InboxItem }) {
  if (item.kind === "permission") return <PermissionCard item={item} />
  if (item.kind === "question") return <QuestionCard item={item} />
  if (item.kind === "pin") return <PinCard item={item} />
  if (item.kind === "failed") return <FailedCard item={item} />
  return <ResultCard item={item} />
}
