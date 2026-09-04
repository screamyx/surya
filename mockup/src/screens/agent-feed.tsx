// Screen: agent feed. Feature 1 - chat with Claude Code: tool calls, diffs, A2UI cards,
// subagents, and the two things that stop the run: a permission ask and a question.
// Round two adds the three controls the daemon actually has: switch model, stop, resume a session.
import { useMemo, useRef, useState } from "react"
import { Link, useParams } from "react-router"
import { motion } from "motion/react"
import { ArrowUp, FolderTree, History, Monitor, Play, Sparkles, Square, StopCircle } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupTextarea } from "@/components/ui/input-group"
import { Command, CommandEmpty, CommandGroup, CommandItem, CommandList } from "@/components/ui/command"
import { SlashPalette } from "@/screens/agent-feed/slash"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { StatusBadge } from "@/components/status"
import { A2UICard } from "@/screens/agent-feed/a2ui-card"
import { ModelSheet, modelName, type Effort } from "@/screens/agent-feed/model-sheet"
import { SessionsSheet } from "@/screens/agent-feed/sessions-sheet"
import { StopDialog } from "@/screens/agent-feed/stop-dialog"
import { SystemLine, type SystemRow } from "@/screens/agent-feed/system-line"
import {
  AssistantEvent, DiffEvent, FeedRow, MailInEvent, MailOutEvent, PermissionEvent, QuestionEvent, SubagentEvent, ThinkingEvent, ToolRow, UserEvent, CommandEvent } from "@/screens/agent-feed/events"
import { pop, stagger } from "@/motion"
import { agents, feed, fmtTime, inbox, type FeedEvent } from "@/data"

// data.ts carries no question event. kite's open question from the inbox, dropped into
// raven's feed after f-10 so the kind is drawn somewhere.
const asked = inbox.find((i) => i.id === "i-2")
const questionEvent: FeedEvent = {
  id: "f-10a",
  kind: "question",
  at: "2026-09-05T01:04:30+08:00",
  question: asked?.title ?? "When should the reminder fire?",
  options: (asked?.options ?? []).map((o) => o.label),
}

const events: FeedEvent[] = (() => {
  const out = [...feed]
  out.splice(out.findIndex((e) => e.id === "f-10") + 1, 0, questionEvent)
  return out
})()

// The mockup's "now". Anything you do in this screen lands at this minute.
const now = "2026-09-05T01:14:00+08:00"

function HeaderStrip({ ws, name, model, summary, status, startedAt, stopped, onStop, onResume, onSessions }: {
  ws: string; name: string; model: string; summary: string
  status: Parameters<typeof StatusBadge>[0]["status"]; startedAt: string
  stopped: boolean; onStop: () => void; onResume: () => void; onSessions: () => void
}) {
  return (
    <div className="bg-background/95 sticky top-12 z-20 border-b backdrop-blur">
      <div className="mx-auto flex max-w-3xl flex-wrap items-start gap-x-3 gap-y-2 px-4 py-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="font-heading text-base font-medium">{name}</h1>
            <StatusBadge status={stopped ? "failed" : status} />
            <Badge variant="outline" className="hidden font-mono md:inline-flex">{model}</Badge>
            <span className="text-muted-foreground hidden text-xs md:inline">started {fmtTime(startedAt)}</span>
          </div>
          <p className="text-muted-foreground mt-1 text-sm">{summary}</p>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          {stopped ? (
            <Button size="sm" variant="outline" onClick={onResume} aria-label="Resume the agent" className="max-sm:size-8 max-sm:px-0">
              <Play data-icon="inline-start" />
              <span className="max-sm:sr-only">Resume</span>
            </Button>
          ) : (
            <Button size="sm" variant="outline" onClick={onStop} aria-label="Stop the agent" className="text-destructive border-destructive/30 hover:bg-destructive/10 max-sm:size-8 max-sm:px-0">
              <Square data-icon="inline-start" />
              <span className="max-sm:sr-only">Stop</span>
            </Button>
          )}
          <Button size="sm" variant="ghost" onClick={onSessions} aria-label="Sessions with this agent" className="max-sm:size-8 max-sm:px-0">
            <History data-icon="inline-start" />
            <span className="max-sm:sr-only">Sessions</span>
          </Button>
          <Button size="sm" variant="ghost" nativeButton={false} render={<Link to={`/w/${ws}/files`} />} className="hidden md:inline-flex">
            <FolderTree data-icon="inline-start" />Open files
          </Button>
          <Button size="sm" variant="ghost" nativeButton={false} render={<Link to={`/w/${ws}/preview`} />} className="hidden md:inline-flex">
            <Monitor data-icon="inline-start" />Open preview
          </Button>
        </div>
      </div>
    </div>
  )
}

function Composer({ model, onModel }: { model: string; onModel: () => void }) {
  const [text, setText] = useState("")
  const ref = useRef<HTMLTextAreaElement>(null)
  const open = text.startsWith("/") && !text.includes(" ")
  const pick = (name: string) => { setText(`/${name} `); ref.current?.focus() }
  // A menu command never becomes text. surya draws the sheet and sends the line for you.
  const menu = () => { setText(""); onModel() }
  return (
    <div className="bg-background/95 sticky bottom-0 z-20 border-t px-4 pt-3 pb-20 backdrop-blur md:pb-3">
      <div className="relative mx-auto max-w-3xl">
        {open && <SlashPalette query={text.slice(1)} onPick={pick} onMenu={menu} Command={Command} CommandList={CommandList} CommandGroup={CommandGroup} CommandItem={CommandItem} CommandEmpty={CommandEmpty} />}
        <InputGroup>
          <InputGroupTextarea ref={ref} name="reply" rows={2} value={text} onChange={(e) => setText(e.target.value)} placeholder="Reply, ask for the next thing, or type / for a command" />
          <InputGroupAddon align="block-end" className="border-t">
            <Badge
              variant="outline"
              className="hover:bg-muted cursor-pointer font-mono"
              render={<button type="button" onClick={onModel} aria-label="Change the model" />}
            >
              model: {model.replace("claude-", "")}
            </Badge>
            <span className="text-muted-foreground hidden items-center gap-1.5 text-xs sm:flex">
              <KbdGroup><Kbd>⌘</Kbd><Kbd>↵</Kbd></KbdGroup> to send · <Kbd>/</Kbd> commands
            </span>
            <InputGroupButton variant="default" size="icon-sm" className="ml-auto" aria-label="Send">
              <ArrowUp />
            </InputGroupButton>
          </InputGroupAddon>
        </InputGroup>
      </div>
    </div>
  )
}

function EventBody({ e, ws, name }: { e: FeedEvent; ws: string; name: string }) {
  switch (e.kind) {
    case "user": return <UserEvent e={e} />
    case "assistant": return <AssistantEvent e={e} name={name} />
    case "thinking": return <ThinkingEvent e={e} />
    case "tool": return <ToolRow t={e} at={e.at} />
    case "diff": return <DiffEvent e={e} ws={ws} />
    case "card": return <A2UICard card={e.card} />
    case "subagent": return <SubagentEvent e={e} />
    case "question": return <QuestionEvent e={e} />
    case "permission": return <PermissionEvent e={e} name={name} />
    case "command": return <CommandEvent e={e} />
    case "mail-out": return <MailOutEvent e={e} />
    case "mail-in": return <MailInEvent e={e} />
  }
}

export function AgentFeedScreen() {
  const { ws = "project-jag", id = "raven" } = useParams()
  // Only raven has a feed in the mockup, so any id lands on raven's transcript.
  const agent = useMemo(() => agents.find((a) => a.id === id) ?? agents[0], [id])
  const [key] = useState(() => `${ws}-${id}`)

  const [model, setModel] = useState(agent.model)
  const [effort, setEffort] = useState<Effort>("high")
  const [modelOpen, setModelOpen] = useState(false)
  const [sessionsOpen, setSessionsOpen] = useState(false)
  const [stopOpen, setStopOpen] = useState(false)
  const [stopped, setStopped] = useState(false)
  const [rows, setRows] = useState<SystemRow[]>([])

  const add = (row: SystemRow) => setRows((r) => [...r, row])

  // The wording is what Claude Code itself replies to "/model <id>", probed 2026-09-05.
  const switchModel = (next: string, level: Effort) => {
    setModel(next)
    setEffort(level)
    setModelOpen(false)
    add({ id: `sys-model-${rows.length}`, text: `Set model to ${modelName(next)} for this session only`, at: now, Icon: Sparkles })
  }

  const stop = () => {
    setStopped(true)
    setStopOpen(false)
    add({ id: `sys-stop-${rows.length}`, text: `Stopped by you at ${fmtTime(now)} · the running Bash was interrupted`, Icon: StopCircle })
  }

  return (
    <div className="flex min-h-[calc(100vh-3rem)] flex-col">
      <HeaderStrip
        ws={ws}
        name={agent.name}
        model={model}
        summary={agent.summary}
        status={agent.status}
        startedAt={agent.startedAt}
        stopped={stopped}
        onStop={() => setStopOpen(true)}
        onResume={() => setStopped(false)}
        onSessions={() => setSessionsOpen(true)}
      />
      <motion.div
        key={key}
        variants={stagger}
        initial="hidden"
        animate="show"
        className="mx-auto w-full max-w-3xl flex-1 space-y-5 px-4 py-6"
      >
        {events.map((e) =>
          e.kind === "permission" ? (
            <motion.div key={e.id} variants={pop}>
              <EventBody e={e} ws={ws} name={agent.name} />
            </motion.div>
          ) : (
            <FeedRow key={e.id}>
              <EventBody e={e} ws={ws} name={agent.name} />
            </FeedRow>
          ),
        )}
        {rows.map((r) => (
          <motion.div key={r.id} variants={pop} initial="hidden" animate="show">
            <SystemLine row={r} />
          </motion.div>
        ))}
      </motion.div>
      <Composer model={model} onModel={() => setModelOpen(true)} />

      <ModelSheet open={modelOpen} onOpenChange={setModelOpen} model={model} effort={effort} onSwitch={switchModel} />
      <SessionsSheet open={sessionsOpen} onOpenChange={setSessionsOpen} name={agent.name} sessions={agent.sessions} />
      <StopDialog open={stopOpen} onOpenChange={setStopOpen} name={agent.name} onConfirm={stop} />
    </div>
  )
}
