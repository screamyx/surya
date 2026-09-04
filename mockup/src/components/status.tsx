import { motion } from "motion/react"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { breathe } from "@/motion"
import type { AgentStatus } from "@/data"

// The three states a person cares about, plus idle. Colours come from tokens only.
// `live` marks the states that have not finished: those breathe, through the motion
// token rather than Tailwind's animate-pulse, so reduced motion is honoured globally
// and the timing lives with the other motion values.
export const statusMeta: Record<AgentStatus, { label: string; dot: string; badge: "default" | "secondary" | "destructive" | "outline"; live?: boolean }> = {
  working: { label: "Working", dot: "bg-primary", badge: "default", live: true },
  "needs-you": { label: "Needs you", dot: "bg-destructive", badge: "destructive" },
  failed: { label: "Stopped", dot: "border-destructive border-2 bg-transparent", badge: "outline" },
  done: { label: "Done", dot: "bg-muted-foreground", badge: "secondary" },
  idle: { label: "Idle", dot: "bg-border-strong", badge: "outline" },
}

export function StatusDot({ status, className }: { status: AgentStatus; className?: string }) {
  const m = statusMeta[status]
  return (
    <motion.span
      aria-label={m.label}
      variants={breathe}
      initial="rest"
      animate={m.live ? "live" : "rest"}
      className={cn("inline-block size-2 shrink-0 rounded-full", m.dot, className)}
    />
  )
}

export function StatusBadge({ status, className }: { status: AgentStatus; className?: string }) {
  const m = statusMeta[status]
  return (
    <Badge variant={m.badge} className={cn("gap-1.5", status === "failed" && "border-destructive text-destructive", className)}>
      <motion.span
        variants={breathe}
        initial="rest"
        animate={m.live ? "live" : "rest"}
        className={cn(
          "size-1.5 rounded-full",
          status === "working" ? "bg-primary-foreground" : status === "needs-you" ? "bg-destructive-foreground" : status === "failed" ? "bg-destructive" : "bg-current"
        )}
      />
      {m.label}
    </Badge>
  )
}
