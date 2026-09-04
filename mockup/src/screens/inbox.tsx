// Screen: /inbox - feature 2, the Needs You inbox with push.
// One column of cards. Every permission ask, question, pin and result lands here.
import { useState } from "react"
import { Link } from "react-router"
import { motion } from "motion/react"
import { Check, ExternalLink, MapPin, Rocket, X } from "lucide-react"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Item, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Textarea } from "@/components/ui/textarea"
import { agentById, fmtTime, inbox, settings, workspaceById, type InboxItem } from "@/data"
import { rise, stagger } from "@/motion"
import { cn } from "@/lib/utils"

type Tab = "all" | "needs" | "pins" | "results"
type Decision = "approved" | "rejected"

const tabFilter: Record<Tab, (i: InboxItem) => boolean> = {
  all: () => true,
  needs: (i) => i.kind === "permission" || i.kind === "question",
  pins: (i) => i.kind === "pin",
  results: (i) => i.kind === "result",
}

const tabLabel: Record<Tab, string> = { all: "All", needs: "Needs you", pins: "Pins", results: "Results" }
const emptyLine: Record<Tab, string> = {
  all: "Nothing waiting. Your agents will ping you here.",
  needs: "No questions right now. Both agents are working.",
  pins: "No pins yet. Tap the preview to leave one.",
  results: "No finished work yet. It shows up here when a PR is ready.",
}

// Phone: thumb sized and full width. Desktop: a normal button row.
const thumb = "h-11 w-full sm:h-8 sm:w-auto"

function CardTop({ item }: { item: InboxItem }) {
  const agent = agentById(item.agentId)
  const ws = workspaceById(item.workspaceId)
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Avatar className="size-7">
        <AvatarFallback className="text-xs uppercase">{agent.name.slice(0, 1)}</AvatarFallback>
      </Avatar>
      <span className="font-medium">{agent.name}</span>
      <Badge variant="outline">{ws.name}</Badge>
      <span className="text-muted-foreground ml-auto text-xs tabular-nums">{fmtTime(item.at)}</span>
    </div>
  )
}

function OpenAgent({ item }: { item: InboxItem }) {
  return (
    <Button variant="ghost" size="sm" nativeButton={false} className="text-muted-foreground" render={<Link to={`/w/${item.workspaceId}/agent/${item.agentId}`} />}>
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

function InboxCard({ item }: { item: InboxItem }) {
  if (item.kind === "permission") return <PermissionCard item={item} />
  if (item.kind === "question") return <QuestionCard item={item} />
  if (item.kind === "pin") return <PinCard item={item} />
  return <ResultCard item={item} />
}

export function InboxScreen() {
  const [tab, setTab] = useState<Tab>("all")
  const [push, setPush] = useState(settings.notifications.push)
  const items = inbox.filter(tabFilter[tab])
  const needs = inbox.filter(tabFilter.needs).length

  return (
    <div className="mx-auto w-full max-w-3xl px-4 py-5 md:px-6 md:py-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h1 className="font-heading text-xl font-semibold tracking-tight">Needs you</h1>
          <p className="text-muted-foreground text-sm">
            {inbox.length} in your inbox, {needs} waiting on an answer.
          </p>
        </div>
        <div className="flex items-start gap-3">
          <div className="text-right">
            <Label htmlFor="push" className="justify-end text-sm">Push notifications</Label>
            <p className="text-muted-foreground text-xs">
              Quiet {settings.notifications.quietFrom} to {settings.notifications.quietTo}
            </p>
          </div>
          <Switch id="push" checked={push} onCheckedChange={setPush} className="mt-1" />
        </div>
      </div>

      <Tabs value={tab} onValueChange={(v) => setTab(v as Tab)} className="mt-5">
        <TabsList className="w-full overflow-x-auto">
          {(Object.keys(tabLabel) as Tab[]).map((t) => {
            const n = inbox.filter(tabFilter[t]).length
            return (
              <TabsTrigger key={t} value={t}>
                {tabLabel[t]}
                <Badge variant={t === "needs" && n > 0 ? "destructive" : "secondary"} className="px-1.5 text-xs">{n}</Badge>
              </TabsTrigger>
            )
          })}
        </TabsList>
      </Tabs>

      {items.length === 0 ? (
        <Empty className="mt-6 min-h-60 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><Check /></EmptyMedia>
            <EmptyTitle>All clear</EmptyTitle>
            <EmptyDescription>{emptyLine[tab]}</EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <motion.div key={tab} variants={stagger} initial="hidden" animate="show" className="mt-5 flex flex-col gap-4">
          {items.map((item) => (
            <motion.div key={item.id} variants={rise}>
              <InboxCard item={item} />
            </motion.div>
          ))}
        </motion.div>
      )}
    </div>
  )
}
