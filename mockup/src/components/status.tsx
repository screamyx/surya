import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import type { AgentStatus } from "@/data"

// The three states a person cares about, plus idle. Colours come from tokens only.
export const statusMeta: Record<AgentStatus, { label: string; dot: string; badge: "default" | "secondary" | "destructive" | "outline" }> = {
  working: { label: "Working", dot: "bg-primary animate-pulse", badge: "default" },
  "needs-you": { label: "Needs you", dot: "bg-destructive", badge: "destructive" },
  failed: { label: "Stopped", dot: "border-destructive border-2 bg-transparent", badge: "outline" },
  done: { label: "Done", dot: "bg-muted-foreground", badge: "secondary" },
  idle: { label: "Idle", dot: "bg-border", badge: "outline" },
}

export function StatusDot({ status, className }: { status: AgentStatus; className?: string }) {
  return <span aria-label={statusMeta[status].label} className={cn("inline-block size-2 shrink-0 rounded-full", statusMeta[status].dot, className)} />
}

export function StatusBadge({ status, className }: { status: AgentStatus; className?: string }) {
  const m = statusMeta[status]
  return (
    <Badge variant={m.badge} className={cn("gap-1.5", status === "failed" && "border-destructive text-destructive", className)}>
      <span className={cn("size-1.5 rounded-full", status === "working" ? "bg-primary-foreground animate-pulse" : status === "needs-you" ? "bg-destructive-foreground" : status === "failed" ? "bg-destructive" : "bg-current")} />
      {m.label}
    </Badge>
  )
}
