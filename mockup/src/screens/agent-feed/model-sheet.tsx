// The /model sheet. A class-2 slash command: surya draws the menu, then sends "/model <id>" as a
// plain turn. Probed 2026-09-05, see docs/probes/headless-controls-2026-09-05.md.
import { useEffect, useState } from "react"
import { motion } from "motion/react"
import { Check } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Item, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item"
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from "@/components/ui/sheet"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { useIsMobile } from "@/hooks/use-mobile"
import { rise, stagger } from "@/motion"
import { settings } from "@/data"
import { cn } from "@/lib/utils"

// Plain words for what each model is for. The dealer picks on cost and speed, not on benchmarks.
const notes: Record<string, string> = {
  "claude-fable-5-1": "most capable, scarce",
  "claude-opus-5": "strong, good for builds",
  "claude-sonnet-5": "fast, cheap, day-to-day",
  "gpt-5.6-sol": "second family, for reviews",
}

const names: Record<string, string> = {
  "claude-fable-5-1": "Fable 5.1",
  "claude-opus-5": "Opus 5",
  "claude-sonnet-5": "Sonnet 5",
  "gpt-5.6-sol": "GPT 5.6 sol",
}

export const modelName = (id: string) => names[id] ?? id
export const efforts = ["low", "medium", "high", "max"] as const
export type Effort = (typeof efforts)[number]

export function ModelSheet({ open, onOpenChange, model, effort, onSwitch }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  model: string
  effort: Effort
  onSwitch: (model: string, effort: Effort) => void
}) {
  const isMobile = useIsMobile()
  const [pick, setPick] = useState(model)
  const [level, setLevel] = useState<Effort>(effort)

  // Reopening always starts from what is actually running, never from a half-made choice.
  useEffect(() => {
    if (open) { setPick(model); setLevel(effort) }
  }, [open, model, effort])

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side={isMobile ? "bottom" : "right"} className={cn("gap-0 p-0", isMobile && "max-h-[85svh]")}>
        <SheetHeader className="border-b">
          <SheetTitle>Model for this agent</SheetTitle>
          <SheetDescription>What raven thinks with from the next turn on.</SheetDescription>
        </SheetHeader>

        <div className="min-h-0 flex-1 overflow-y-auto p-4">
          <motion.div
            key={open ? "open" : "shut"}
            variants={stagger}
            initial="hidden"
            animate="show"
            role="radiogroup"
            aria-label="Model"
            className="grid gap-2"
          >
            {settings.models.map((id) => (
              <motion.div key={id} variants={rise}>
                <ModelRow id={id} picked={pick === id} current={model === id} onPick={setPick} />
              </motion.div>
            ))}
          </motion.div>

          <div className="mt-6 space-y-2">
            <p className="text-sm font-medium">Effort</p>
            <p className="text-muted-foreground text-sm">How long it may think before it answers.</p>
            <ToggleGroup
              value={[level]}
              onValueChange={(v) => v.length && setLevel(v[0] as Effort)}
              variant="outline"
              spacing={0}
              className="w-full"
            >
              {efforts.map((e) => (
                <ToggleGroupItem key={e} value={e} className="flex-1 capitalize">{e}</ToggleGroupItem>
              ))}
            </ToggleGroup>
          </div>
        </div>

        <SheetFooter className="border-t">
          <p className="text-muted-foreground text-xs">
            surya sends <code className="font-mono">/model {pick}</code> to the agent. It applies to this session only.
          </p>
          <Button size="lg" onClick={() => onSwitch(pick, level)}>Switch</Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  )
}

function ModelRow({ id, picked, current, onPick }: {
  id: string; picked: boolean; current: boolean; onPick: (id: string) => void
}) {
  return (
    <Item
      variant="outline"
      role="radio"
      tabIndex={0}
      aria-checked={picked}
      onClick={() => onPick(id)}
      onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onPick(id) } }}
      className={cn("hover:bg-muted cursor-pointer items-center gap-3", picked && "border-primary bg-muted")}
    >
      <span className={cn("size-4 shrink-0 rounded-full border", picked && "border-primary bg-primary")}>
        {picked && <span className="bg-primary-foreground m-1 block size-2 rounded-full" />}
      </span>
      <ItemContent className="gap-0.5">
        <ItemTitle className="font-mono text-[13px]">{id}</ItemTitle>
        <ItemDescription>{notes[id]}</ItemDescription>
      </ItemContent>
      {current && (
        <span className="flex shrink-0 items-center gap-1.5">
          <Check className="text-muted-foreground size-4" />
          <Badge variant="secondary">current</Badge>
        </span>
      )}
    </Item>
  )
}
