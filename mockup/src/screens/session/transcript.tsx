// The conversation with one agent, and the only pane that is always on screen.
// Machine work folds into "Worked for 4m 12s" (see run-fold.tsx). A permission card and a
// question stay open at full weight, because they are the two things waiting on a person.
import { useMemo } from "react"
import { motion } from "motion/react"
import { A2UICard } from "@/screens/session/a2ui-card"
import { Composer } from "@/screens/session/composer"
import { RunFold, strands } from "@/screens/session/run-fold"
import { SystemLine } from "@/screens/session/system-line"
import { useSessionControls } from "@/screens/session/controls"
import {
  AssistantEvent, CommandEvent, DiffEvent, FeedRow, MailInEvent, MailOutEvent,
  PermissionEvent, QuestionEvent, SubagentEvent, ThinkingEvent, ToolRow, UserEvent,
} from "@/screens/session/events"
import { pop, stagger } from "@/motion"
import { feed, inbox, type Agent, type FeedEvent, type Workspace } from "@/data"
import { ScrollArea } from "@/components/ui/scroll-area"

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
    case "permission": return <PermissionEvent e={e} name={name} ws={ws} />
    case "command": return <CommandEvent e={e} />
    case "mail-out": return <MailOutEvent e={e} />
    case "mail-in": return <MailInEvent e={e} />
  }
}

export function Transcript({ agent, ws }: { agent: Agent; ws: Workspace }) {
  const controls = useSessionControls()
  const parts = useMemo(() => strands(events), [])

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      <ScrollArea className="min-h-0 flex-1">
        <motion.div
          key={agent.id}
          variants={stagger}
          initial="hidden"
          animate="show"
          className="mx-auto w-full max-w-3xl space-y-5 px-4 py-6"
        >
          {parts.map((s) =>
            s.kind === "run" ? (
              <RunFold key={s.run.id} run={s.run}>
                {s.run.events.map((e) => (
                  <div key={e.id}>
                    <EventBody e={e} ws={ws.id} name={agent.name} />
                  </div>
                ))}
              </RunFold>
            ) : s.event.kind === "permission" ? (
              <motion.div key={s.event.id} variants={pop}>
                <EventBody e={s.event} ws={ws.id} name={agent.name} />
              </motion.div>
            ) : (
              <FeedRow key={s.event.id}>
                <EventBody e={s.event} ws={ws.id} name={agent.name} />
              </FeedRow>
            ),
          )}
          {controls.rows.map((r) => (
            <motion.div key={r.id} variants={pop} initial="hidden" animate="show">
              <SystemLine row={r} />
            </motion.div>
          ))}
        </motion.div>
      </ScrollArea>

      <Composer model={controls.model} onModel={() => controls.setModelOpen(true)} />
    </div>
  )
}
