// Pure helpers behind the pin overlay, kept out of pins.tsx so that file exports only
// components.
import type { Pin } from "@/data"

// Open by you is loud, agent pins are the primary colour, anything already handled goes quiet.
export function markerTone(p: Pin) {
  if (p.status === "fixed") return "bg-muted-foreground text-background"
  if (p.status === "taken") return "bg-secondary text-secondary-foreground"
  if (p.by === "agent") return "bg-primary text-primary-foreground"
  return "bg-destructive text-destructive-foreground"
}

export const statusLabel: Record<Pin["status"], string> = {
  open: "Open",
  taken: "Agent took it",
  fixed: "Fixed",
}

export const statusVariant: Record<Pin["status"], "destructive" | "secondary" | "outline"> = {
  open: "destructive",
  taken: "secondary",
  fixed: "outline",
}

// What the agent receives: where you pointed, what it is, and what you said about it.
export const asPrompt = (p: Pin, n: number, url: string) =>
  `Pin ${n} on ${url}\n${p.selector}\n\n${p.note}\n`

// The daemon names the element under the click. Chrome answers with a stable selector for
// the element, not with whatever utility classes the page happens to wear, so the pages in
// this pane carry a `data-el` hook and this reads it.
export function selectorAt(el: Element | null): string {
  const hit = el?.closest("[data-el]")
  if (hit) return hit.getAttribute("data-el") ?? "body"
  return el ? el.tagName.toLowerCase() : "body"
}
