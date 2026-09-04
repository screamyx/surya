// Screen: agent feed. Feature 1 - chat with Claude Code: tool calls, diffs, A2UI cards,
// subagents, and the two things that stop the run: a permission ask and a question.
import { useMemo, useState } from "react"
import { Link, useParams } from "react-router"
import { motion } from "motion/react"
import { ArrowUp, FolderTree, Monitor, Square } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupTextarea } from "@/components/ui/input-group"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { StatusBadge } from "@/components/status"
import { A2UICard } from "@/screens/agent-feed/a2ui-card"
import {
  AssistantEvent, DiffEvent, FeedRow, PermissionEvent, QuestionEvent, SubagentEvent, ThinkingEvent, ToolRow, UserEvent,
} from "@/screens/agent-feed/events"
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

function HeaderStrip({ ws, name, model, summary, status, startedAt }: {
  ws: string; name: string; model: string; summary: string
  status: Parameters<typeof StatusBadge>[0]["status"]; startedAt: string
}) {
  return (
    <div className="bg-background/95 sticky top-12 z-20 border-b backdrop-blur">
      <div className="mx-auto flex max-w-3xl flex-wrap items-start gap-x-3 gap-y-2 px-4 py-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="font-heading text-base font-medium">{name}</h1>
            <StatusBadge status={status} />
            <Badge variant="outline" className="hidden font-mono md:inline-flex">{model}</Badge>
            <span className="text-muted-foreground hidden text-xs md:inline">started {fmtTime(startedAt)}</span>
          </div>
          <p className="text-muted-foreground mt-1 text-sm">{summary}</p>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <Button size="sm" variant="outline" aria-label="Stop the agent" className="text-destructive border-destructive/30 hover:bg-destructive/10">
            <Square data-icon="inline-start" />
            <span className="max-sm:sr-only">Stop</span>
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

function Composer({ model }: { model: string }) {
  return (
    <div className="bg-background/95 sticky bottom-0 z-20 border-t px-4 pt-3 pb-20 backdrop-blur md:pb-3">
      <div className="mx-auto max-w-3xl">
        <InputGroup>
          <InputGroupTextarea name="reply" rows={2} placeholder="Reply, or ask for the next thing" />
          <InputGroupAddon align="block-end" className="border-t">
            <Badge variant="outline" className="font-mono">model: {model.replace("claude-", "")}</Badge>
            <span className="text-muted-foreground hidden items-center gap-1.5 text-xs sm:flex">
              <KbdGroup><Kbd>⌘</Kbd><Kbd>↵</Kbd></KbdGroup> to send
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
  }
}

export function AgentFeedScreen() {
  const { ws = "project-jag", id = "raven" } = useParams()
  // Only raven has a feed in the mockup, so any id lands on raven's transcript.
  const agent = useMemo(() => agents.find((a) => a.id === id) ?? agents[0], [id])
  const [key] = useState(() => `${ws}-${id}`)

  return (
    <div className="flex min-h-[calc(100vh-3rem)] flex-col">
      <HeaderStrip
        ws={ws}
        name={agent.name}
        model={agent.model}
        summary={agent.summary}
        status={agent.status}
        startedAt={agent.startedAt}
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
      </motion.div>
      <Composer model={agent.model} />
    </div>
  )
}
