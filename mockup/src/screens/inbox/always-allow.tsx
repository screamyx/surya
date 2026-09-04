// The third, quieter action on a permission ask: answer it once and keep the answer.
// Approve stays the one accent button; this is a ghost, never a coloured fill.
// Clicking it shows the rule in plain words and lets you narrow the scope first,
// the way Kiro's Always allow opens a pattern and scope picker before it commits.
import { useState } from "react"
import { Link } from "react-router"
import { ShieldCheck } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Label } from "@/components/ui/label"
import { Popover, PopoverContent, PopoverDescription, PopoverHeader, PopoverTitle, PopoverTrigger } from "@/components/ui/popover"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { Separator } from "@/components/ui/separator"
import type { InboxItem } from "@/data"
import { ruleDraft, type CreatedRule } from "@/screens/inbox/rule-draft"
import { cn } from "@/lib/utils"

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5">
      <p className="u-overline text-muted-foreground">{label}</p>
      {children}
    </div>
  )
}

// One radio row: the machine text leads, the plain-words hint sits under it.
function Choice({ id, value, mono, hint }: { id: string; value: string; mono: string; hint: string }) {
  return (
    <Label htmlFor={id} className="hover:bg-muted/60 -mx-1.5 flex items-start gap-2.5 rounded-md px-1.5 py-1.5">
      <RadioGroupItem id={id} value={value} className="mt-0.5" />
      <span className="flex min-w-0 flex-col gap-0.5">
        <span className="font-mono text-xs font-medium [overflow-wrap:anywhere]">{mono}</span>
        <span className="text-muted-foreground text-xs font-normal">{hint}</span>
      </span>
    </Label>
  )
}

export function AlwaysAllow({ item, onCreate, className }: {
  item: InboxItem
  onCreate: (rule: CreatedRule) => void
  className?: string
}) {
  const draft = ruleDraft(item)
  const [open, setOpen] = useState(false)
  const [pattern, setPattern] = useState(draft.patterns[0].id)
  const [scope, setScope] = useState(draft.scopes[0].id)

  const picked = draft.patterns.find((p) => p.id === pattern)!
  const place = draft.scopes.find((s) => s.id === scope)!

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        render={
          <Button variant="ghost" className={cn("text-muted-foreground hover:text-foreground h-11 sm:h-8", className)}>
            <ShieldCheck data-icon="inline-start" />
            Always allow {draft.noun} in {draft.scopes[0].scope}
          </Button>
        }
      />
      <PopoverContent align="end" className="w-72 gap-3">
        <PopoverHeader>
          <PopoverTitle>Always allow {draft.noun}</PopoverTitle>
          <PopoverDescription>
            Approve this now and answer every later ask that matches, without asking you again.
          </PopoverDescription>
        </PopoverHeader>

        <Field label="Matches">
          <RadioGroup value={pattern} onValueChange={(v) => setPattern(v as typeof pattern)} className="gap-0">
            {draft.patterns.map((p) => (
              <Choice key={p.id} id={`pat-${item.id}-${p.id}`} value={p.id} mono={p.pattern} hint={p.hint} />
            ))}
          </RadioGroup>
        </Field>

        <Field label="Applies to">
          <RadioGroup value={scope} onValueChange={(v) => setScope(v as typeof scope)} className="gap-0">
            {draft.scopes.map((s) => (
              <Choice key={s.id} id={`scope-${item.id}-${s.id}`} value={s.id} mono={s.label} hint={s.hint} />
            ))}
          </RadioGroup>
        </Field>

        <Separator />
        <p className="text-muted-foreground text-xs">
          {draft.capabilityLabel} matching{" "}
          <span className="text-foreground font-mono [overflow-wrap:anywhere]">{picked.pattern}</span>, in{" "}
          <span className="text-foreground font-mono">{place.scope}</span>.
        </p>
        <div className="flex gap-2">
          <Button
            className="h-9 flex-1 sm:h-8"
            onClick={() => {
              setOpen(false)
              onCreate({ capabilityLabel: draft.capabilityLabel, pattern: picked.pattern, scope: place.scope })
            }}
          >
            Create rule
          </Button>
          <Button variant="outline" className="h-9 sm:h-8" onClick={() => setOpen(false)}>
            Cancel
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  )
}

// What the card shows once the rule exists. It names the rule exactly, because the
// mockup carries no live rule list to point at.
export function RuleMade({ rule, onUndo }: { rule: CreatedRule; onUndo: () => void }) {
  return (
    <div className="bg-muted flex w-full flex-col gap-1.5 rounded-lg p-3">
      <p className="flex items-center gap-2 text-sm font-medium">
        <ShieldCheck className="text-ok size-4 shrink-0" />
        Approved, and the answer is kept
      </p>
      <p className="font-mono text-xs [overflow-wrap:anywhere]">
        {rule.capabilityLabel} · {rule.pattern} · {rule.scope}
      </p>
      <div className="flex flex-wrap items-center gap-x-3">
        <Button variant="link" className="h-8 px-0" nativeButton={false} render={<Link to="/settings" />}>
          See approval rules
        </Button>
        <Button variant="ghost" className="text-muted-foreground h-8 px-2" onClick={onUndo}>
          Undo
        </Button>
      </div>
    </div>
  )
}
