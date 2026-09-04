// The session header. The agent's name is the first place the eye lands on this surface,
// so it carries the serif display face at a real type step above everything under it.
import { History, Play, Square } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { StatusBadge } from "@/components/status"
import { fmtTime, type Agent, type Workspace } from "@/data"
import { modelName } from "@/screens/session/model-sheet"

export function SessionHeader({ agent, ws, model, stopped, onStop, onResume, onSessions }: {
  agent: Agent
  ws: Workspace
  model: string
  stopped: boolean
  onStop: () => void
  onResume: () => void
  onSessions: () => void
}) {
  // An agent that already stopped on its own is not something you can stop again.
  const halted = stopped || agent.status === "failed"
  return (
    <header className="bg-background shrink-0 border-b px-4 py-3 md:px-6">
      <div className="flex min-w-0 flex-wrap items-baseline gap-x-3">
        <h1 className="u-display min-w-0 truncate text-4xl">{agent.name}</h1>
        <span className="text-muted-foreground min-w-0 truncate font-mono text-xs">
          {ws.name} · {ws.branch}
        </span>
      </div>
      <p className="text-ink-soft mt-1 max-w-[65ch] text-sm">{agent.summary}</p>

      {/* State and the two controls share a row, so the header costs one line less on a phone. */}
      <div className="mt-2 flex flex-wrap items-center gap-x-2 gap-y-1.5">
        <StatusBadge status={halted ? "failed" : agent.status} />
        <Badge variant="outline" className="font-mono">{modelName(model)}</Badge>
        <span className="text-muted-foreground text-xs tnum">started {fmtTime(agent.startedAt)}</span>
        <div className="ml-auto flex shrink-0 items-center gap-1.5">
          {halted ? (
            <Button size="sm" variant="outline" onClick={onResume}>
              <Play data-icon="inline-start" />Resume
            </Button>
          ) : (
            <Button
              size="sm"
              variant="outline"
              onClick={onStop}
              className="text-destructive border-destructive/30 hover:bg-destructive/10"
            >
              <Square data-icon="inline-start" className="size-3.5 fill-current" />Stop
            </Button>
          )}
          <Button size="sm" variant="ghost" onClick={onSessions}>
            <History data-icon="inline-start" />Sessions
          </Button>
        </div>
      </div>
    </header>
  )
}
