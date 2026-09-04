// One renderer per FeedEvent kind. The feed screen maps over these; nothing here fetches.
import { useState } from "react"
import { Link } from "react-router"
import { motion } from "motion/react"
import { Check, ChevronRight, FileDiff, Mail, Send, ShieldAlert, ShieldCheck, Sparkles, X } from "lucide-react"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import { Item } from "@/components/ui/item"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Textarea } from "@/components/ui/textarea"
import { fade, rise } from "@/motion"
import { fmtTime, inbox, type FeedEvent } from "@/data"
import { cn } from "@/lib/utils"

type Of<K extends FeedEvent["kind"]> = Extract<FeedEvent, { kind: K }>

function Stamp({ at, className }: { at: string; className?: string }) {
  return <span className={cn("text-muted-foreground text-xs tabular-nums", className)}>{fmtTime(at)}</span>
}

export function UserEvent({ e }: { e: Of<"user"> }) {
  return (
    <div className="flex flex-col items-end gap-1">
      <div className="bg-primary text-primary-foreground max-w-[85%] rounded-2xl px-4 py-2.5 text-sm leading-relaxed">{e.text}</div>
      <Stamp at={e.at} />
    </div>
  )
}

export function AssistantEvent({ e, name }: { e: Of<"assistant">; name: string }) {
  return (
    <div className="flex gap-3">
      <Avatar className="size-6 shrink-0"><AvatarFallback className="text-xs uppercase">{name.slice(0, 1)}</AvatarFallback></Avatar>
      <div className="min-w-0 space-y-1">
        <p className="text-sm leading-relaxed">{e.text}</p>
        <Stamp at={e.at} />
      </div>
    </div>
  )
}

export function ThinkingEvent({ e }: { e: Of<"thinking"> }) {
  const [open, setOpen] = useState(false)
  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger className="text-muted-foreground hover:text-foreground flex items-center gap-1.5 text-xs transition-colors">
        <Sparkles className="size-3.5" />
        Thought for 4s
        <ChevronRight className={cn("size-3 transition-transform", open && "rotate-90")} />
      </CollapsibleTrigger>
      <CollapsibleContent>
        <p className="text-muted-foreground border-l pt-2 pl-3 text-sm italic">{e.text}</p>
      </CollapsibleContent>
    </Collapsible>
  )
}

type ToolLike = { tool: string; input: string; output: string; ms: number; ok: boolean }

export function ToolRow({ t, at }: { t: ToolLike; at?: string }) {
  const [open, setOpen] = useState(false)
  const bash = t.tool === "Bash"
  return (
    <div className="space-y-2">
      <Item
        variant="outline"
        size="sm"
        role="button"
        tabIndex={0}
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
        onKeyDown={(ev) => { if (ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); setOpen((v) => !v) } }}
        className="hover:bg-muted flex-nowrap gap-2 py-2 cursor-pointer"
      >
        <ChevronRight className={cn("text-muted-foreground size-3.5 shrink-0 transition-transform", open && "rotate-90")} />
        <Badge variant="outline" className="shrink-0 font-mono">{t.tool}</Badge>
        <span className="text-muted-foreground min-w-0 flex-1 truncate font-mono text-xs">{t.input}</span>
        {at && <Stamp at={at} className="hidden sm:inline" />}
        <span className="text-muted-foreground shrink-0 text-xs tabular-nums">{t.ms}ms</span>
        {t.ok ? <Check className="text-muted-foreground size-3.5 shrink-0" /> : <X className="text-destructive size-3.5 shrink-0" />}
      </Item>
      {open && (
        <motion.div variants={fade} initial="hidden" animate="show">
          <ScrollArea className={cn("max-h-64 rounded-md", bash ? "bg-muted" : "border")}>
            <pre className="p-3 font-mono text-xs leading-relaxed break-words whitespace-pre-wrap">
              {bash && <span className="text-muted-foreground block">$ {t.input}</span>}
              {t.output}
            </pre>
          </ScrollArea>
        </motion.div>
      )}
    </div>
  )
}

export function DiffEvent({ e, ws }: { e: Of<"diff">; ws: string }) {
  return (
    <Card size="sm">
      <CardHeader className="flex flex-wrap items-center gap-2">
        <FileDiff className="text-muted-foreground size-4 shrink-0" />
        <span className="min-w-0 flex-1 truncate font-mono text-xs">{e.file}</span>
        <Badge variant="secondary">+{e.added}</Badge>
        <Badge variant="outline">-{e.removed}</Badge>
        <Button size="xs" variant="ghost" nativeButton={false} render={<Link to={`/w/${ws}/files`} />}>Open in editor</Button>
      </CardHeader>
      <CardContent className="px-0">
        <ScrollArea className="w-full border-y">
          <div className="min-w-max py-1 font-mono text-xs leading-relaxed">
            {e.hunk.split("\n").map((line, i) => (
              <div
                key={i}
                className={cn(
                  "px-4 whitespace-pre",
                  line.startsWith("@@") && "text-muted-foreground bg-muted/50",
                  line.startsWith("+") && "bg-accent text-accent-foreground font-medium",
                  line.startsWith("-") && "bg-muted text-muted-foreground line-through",
                  !line.startsWith("@@") && !line.startsWith("+") && !line.startsWith("-") && "text-muted-foreground",
                )}
              >
                {line || " "}
              </div>
            ))}
          </div>
        </ScrollArea>
      </CardContent>
      <CardFooter className="justify-between">
        <span className="text-muted-foreground text-xs">Written by the agent</span>
        <Stamp at={e.at} />
      </CardFooter>
    </Card>
  )
}

// Three plausible nested calls, read off the subagent's own summary.
function nestedCalls(summary: string): ToolLike[] {
  const files = summary.split(":").pop()?.split(",").map((s) => s.trim()).filter(Boolean) ?? []
  const rows: ToolLike[] = files.slice(0, 2).map((f, i) => ({
    tool: "Read", input: f, output: `${f}\nread, ${18 + i * 7} lines`, ms: 11 + i * 7, ok: true,
  }))
  rows.splice(1, 0, { tool: "Grep", input: "follow_up in backend/", output: `${files.join("\n")}`, ms: 96, ok: true })
  return rows
}

export function SubagentEvent({ e }: { e: Of<"subagent"> }) {
  const [open, setOpen] = useState(false)
  return (
    <div className="border-l pl-4">
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant="outline">{e.name}</Badge>
        <span className="text-muted-foreground text-xs">subagent</span>
        <Stamp at={e.at} className="ml-auto" />
      </div>
      <p className="mt-1.5 text-sm leading-relaxed">{e.summary}</p>
      <Collapsible open={open} onOpenChange={setOpen} className="mt-1.5">
        <CollapsibleTrigger className="text-muted-foreground hover:text-foreground flex items-center gap-1.5 text-xs underline underline-offset-4 transition-colors">
          {e.events} events
          <ChevronRight className={cn("size-3 transition-transform", open && "rotate-90")} />
        </CollapsibleTrigger>
        <CollapsibleContent>
          <div className="space-y-1.5 pt-2">
            {nestedCalls(e.summary).map((t, i) => <ToolRow key={i} t={t} />)}
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  )
}

// Decision 20, point 3: an answer can become a rule, so the next identical ask never comes.
// Cline "exposes approval policy by capability" and Kiro "lets one permission decision become
// a precise rule by pattern and scope". The rule lives in Settings once you make it.
const ruleFor = (command: string) => {
  if (command.includes("migrate")) return "migrations"
  if (command.includes("pest") || command.includes("test")) return "test runs"
  return "this command"
}

export function PermissionEvent({ e, name, ws }: { e: Of<"permission">; name: string; ws: string }) {
  const ask = inbox.find((i) => i.id === "i-1")
  const [decided, setDecided] = useState<"approved" | "rejected" | "always" | undefined>(e.decided)
  const rule = `Always allow ${ruleFor(e.command)} in ${ws}`
  return (
    // Loud while it waits on you, and quiet the moment you answer it.
    <Card className={cn(!decided && "ring-destructive/30 ring-2")}>
      <CardHeader>
        <div className="flex flex-wrap items-center gap-2">
          <ShieldAlert className="text-destructive size-4 shrink-0" />
          <CardTitle>{name} wants to run</CardTitle>
          <Badge variant="outline" className="font-mono">{e.tool}</Badge>
          <Stamp at={e.at} className="ml-auto" />
        </div>
        <CardDescription className="pt-1">{ask?.body}</CardDescription>
      </CardHeader>
      <CardContent>
        <pre className="bg-muted rounded-md p-3 font-mono text-xs break-all whitespace-pre-wrap">{e.command}</pre>
      </CardContent>
      <CardFooter className="flex-wrap gap-2 max-sm:flex-col max-sm:items-stretch">
        {decided ? (
          <>
            <Badge variant={decided === "rejected" ? "destructive" : "secondary"} className="h-7 px-3">
              {decided === "rejected" ? "You rejected this" : "You approved this"}
            </Badge>
            {decided === "always" && (
              <span className="text-muted-foreground flex items-center gap-1.5 text-xs">
                <ShieldCheck className="size-3.5" />
                Rule saved: {rule}. Change it in Settings.
              </span>
            )}
          </>
        ) : (
          <>
            <Button size="lg" className="max-sm:w-full" onClick={() => setDecided("approved")}>Approve</Button>
            <Button size="lg" variant="outline" className="max-sm:w-full" onClick={() => setDecided("rejected")}>Reject</Button>
            {/* The quieter second answer. One click here and this ask stops coming back. */}
            <Button
              size="sm"
              variant="ghost"
              className="text-muted-foreground hover:text-foreground max-sm:w-full sm:ml-auto"
              onClick={() => setDecided("always")}
            >
              <ShieldCheck data-icon="inline-start" />{rule}
            </Button>
          </>
        )}
      </CardFooter>
    </Card>
  )
}

export function QuestionEvent({ e }: { e: Of<"question"> }) {
  const src = inbox.find((i) => i.id === "i-2")
  const [picked, setPicked] = useState<string | undefined>(e.answered)
  const describe = (label: string) => src?.options?.find((o) => o.label === label)?.description
  return (
    <Card size="sm">
      <CardHeader>
        <div className="flex flex-wrap items-center gap-2">
          <CardTitle className="min-w-0 flex-1 max-sm:basis-full">{e.question}</CardTitle>
          {picked ? <Badge variant="secondary">Answered: {picked}</Badge> : <Badge variant="outline">Waiting on you</Badge>}
          <Stamp at={e.at} />
        </div>
        <CardDescription className="pt-1">{src?.body}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-2">
        {e.options.map((o) => (
          <Item
            key={o}
            variant="outline"
            size="sm"
            role="radio"
            tabIndex={0}
            aria-checked={picked === o}
            onClick={() => setPicked(o)}
            onKeyDown={(ev) => { if (ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); setPicked(o) } }}
            className={cn("hover:bg-muted cursor-pointer items-start gap-3", picked === o && "border-primary bg-muted")}
          >
            <span className={cn("mt-0.5 size-4 shrink-0 rounded-full border", picked === o && "border-primary bg-primary")}>
              {picked === o && <span className="bg-primary-foreground m-1 block size-2 rounded-full" />}
            </span>
            <span className="min-w-0 flex-1">
              <span className="block text-sm font-medium">{o}</span>
              <span className="text-muted-foreground block text-sm">{describe(o)}</span>
            </span>
          </Item>
        ))}
        <Textarea name="other-answer" className="min-h-16" placeholder="Other: type the time you want" />
      </CardContent>
      <CardFooter>
        <Button size="sm" disabled={!picked}>Send the answer</Button>
      </CardFooter>
    </Card>
  )
}

export function FeedRow({ children }: { children: React.ReactNode }) {
  return <motion.div variants={rise}>{children}</motion.div>
}

// A slash command you sent. Reads like the terminal: the command as a chip, the skill it resolved to underneath.
export function CommandEvent({ e }: { e: Of<"command"> }) {
  return (
    <div className="flex flex-col items-end gap-1">
      <div className="bg-primary text-primary-foreground max-w-[85%] rounded-2xl px-3 py-2 text-sm">
        <span className="bg-primary-foreground text-primary rounded-md px-1.5 py-0.5 font-mono text-xs">/{e.name}</span>
        {e.args && <span className="ml-2">{e.args}</span>}
      </div>
      <div className="text-muted-foreground flex items-center gap-1.5 text-xs">
        <Badge variant="outline" className="h-4 px-1.5 text-[10px]">{e.source}</Badge>
        <span className="truncate">{e.description}</span>
        <Stamp at={e.at} />
      </div>
    </div>
  )
}

// Mail this agent sent. Compact like a tool row: the address, the first line, the delivery id.
export function MailOutEvent({ e }: { e: Of<"mail-out"> }) {
  const [open, setOpen] = useState(false)
  return (
    <div className="space-y-2">
      <Item
        variant="outline"
        size="sm"
        role="button"
        tabIndex={0}
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
        onKeyDown={(ev) => { if (ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); setOpen((v) => !v) } }}
        className="hover:bg-muted flex-nowrap gap-2 py-2 cursor-pointer"
      >
        <ChevronRight className={cn("text-muted-foreground size-3.5 shrink-0 transition-transform", open && "rotate-90")} />
        <Send className="text-muted-foreground size-3.5 shrink-0" />
        <span className="shrink-0 text-xs">sent to <span className="font-medium">{e.to}</span></span>
        <span className="text-muted-foreground min-w-0 flex-1 truncate text-xs">{e.text}</span>
        <Stamp at={e.at} className="hidden sm:inline" />
        <span className="text-muted-foreground shrink-0 font-mono text-xs">{e.deliveryId}</span>
        {e.delivered && (
          <span className="text-muted-foreground flex shrink-0 items-center gap-1 text-xs">
            <Check className="size-3.5" />
            <span className="hidden sm:inline">delivered</span>
          </span>
        )}
      </Item>
      {open && (
        <motion.div variants={fade} initial="hidden" animate="show">
          <p className="text-muted-foreground border-l pl-3 text-sm leading-relaxed">{e.text}</p>
        </motion.div>
      )}
    </div>
  )
}

// Mail that arrived. The daemon delivered it as this agent's next turn, so it reads in full.
export function MailInEvent({ e }: { e: Of<"mail-in"> }) {
  const [acked, setAcked] = useState(e.acked)
  return (
    <Card size="sm">
      <CardHeader className="flex flex-wrap items-center gap-2">
        <Mail className="text-muted-foreground size-4 shrink-0" />
        <CardTitle className="text-sm">from {e.from}</CardTitle>
        <Badge variant="outline" className="font-mono text-xs">{e.deliveryId}</Badge>
        <Stamp at={e.at} className="ml-auto" />
      </CardHeader>
      <CardContent>
        <p className="text-sm leading-relaxed">{e.text}</p>
      </CardContent>
      <CardFooter className="justify-between gap-2">
        <span className="text-muted-foreground text-xs">Arrived as this agent's next turn</span>
        {acked ? (
          <Badge variant="secondary" className="gap-1"><Check />acked</Badge>
        ) : (
          <Button size="xs" variant="outline" onClick={() => setAcked(true)}>Ack</Button>
        )}
      </CardFooter>
    </Card>
  )
}
