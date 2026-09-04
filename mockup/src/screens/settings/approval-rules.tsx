// The specific rules an inbox answer created. Pattern, scope, and how many asks each one
// has answered since. Revoking is destructive, so it wears the destructive variant in the
// trigger and in the confirm alike.
//
// Revoking is the one thing on this panel that changes it, so it is the one thing that
// moves: the rule leaves the list it was in and lands in the revoked strip under it,
// and the rules that stay close the gap on a layout spring rather than jumping.
import { useState } from "react"
import { AnimatePresence, motion } from "motion/react"
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription,
  AlertDialogFooter, AlertDialogHeader, AlertDialogMedia, AlertDialogTitle } from "@/components/ui/alert-dialog"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Item, ItemContent, ItemDescription, ItemGroup, ItemTitle } from "@/components/ui/item"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { ShieldOff } from "lucide-react"
import { approvalRules, type ApprovalRule } from "@/settings-data"
import { layoutSpring, row } from "@/motion"
import { BeatCount } from "@/screens/beat"
import { cn } from "@/lib/utils"

const day = (iso: string) => new Date(iso).toLocaleDateString("en-MY", { day: "numeric", month: "short" })

// A table row that can slide and leave. motion.create wraps the stock TableRow rather
// than restyling a tr by hand, so the table keeps every class it shipped with.
const MotionRow = motion.create(TableRow)

function RevokeButton({ rule, onConfirm, className }: { rule: ApprovalRule; onConfirm: () => void; className?: string }) {
  const [open, setOpen] = useState(false)
  return (
    <>
      <Button variant="destructive" size="sm" className={cn("h-9 md:h-7", className)} onClick={() => setOpen(true)}>
        Revoke
      </Button>
      <AlertDialog open={open} onOpenChange={setOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogMedia className="text-destructive"><ShieldOff /></AlertDialogMedia>
            <AlertDialogTitle>Revoke this rule?</AlertDialogTitle>
            <AlertDialogDescription>
              {rule.label}, <span className="font-mono">{rule.pattern}</span> in{" "}
              <span className="font-mono">{rule.scope}</span>, has answered <span className="tnum">{rule.uses}</span> asks.
              Revoke it and every one of those comes back to your inbox.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep it</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={onConfirm}>Revoke</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  )
}

// Where a revoked rule lands. It names the rule and offers the way back, so a rule that
// leaves the list never leaves without an account of itself.
function RevokedStrip({ rules, onUndo }: { rules: ApprovalRule[]; onUndo: (id: string) => void }) {
  return (
    <motion.div layout="position" transition={layoutSpring} className="border-border flex flex-col gap-1 rounded-lg border border-dashed p-3">
      <p className="text-muted-foreground text-xs">
        Revoked. Their asks come back to your inbox until you allow them again.
      </p>
      <AnimatePresence initial={false}>
        {rules.map((r) => (
          <motion.div
            key={r.id}
            layout="position"
            transition={layoutSpring}
            variants={row}
            initial="hidden"
            animate="show"
            exit="exit"
            className="flex flex-wrap items-center gap-x-3 gap-y-1 overflow-hidden py-1"
          >
            <span className="text-muted-foreground text-sm line-through">{r.label}</span>
            <span className="text-muted-foreground font-mono text-xs [overflow-wrap:anywhere]">{r.pattern}</span>
            <Badge variant="secondary" className="ml-auto shrink-0">Revoked</Badge>
            <Button variant="ghost" size="sm" className="h-9 shrink-0 md:h-7" onClick={() => onUndo(r.id)}>Undo</Button>
          </motion.div>
        ))}
      </AnimatePresence>
    </motion.div>
  )
}

export function RulesList() {
  const [revoked, setRevoked] = useState<string[]>([])
  const gone = (id: string) => revoked.includes(id)
  const revoke = (id: string) => setRevoked((prev) => [...prev, id])
  const undo = (id: string) => setRevoked((prev) => prev.filter((r) => r !== id))
  const live = approvalRules.filter((r) => !gone(r.id))
  // Revoked rules keep the order they were revoked in, so the strip reads as a history.
  const dead = revoked.map((id) => approvalRules.find((r) => r.id === id)!).filter(Boolean)
  const answered = live.reduce((n, r) => n + r.uses, 0)

  return (
    <motion.div layout="position" transition={layoutSpring} className="flex flex-col gap-3">
      <div>
        <p className="text-sm font-medium">Rules you kept</p>
        <p className="text-muted-foreground text-sm">
          <BeatCount value={live.length} /> {live.length === 1 ? "rule" : "rules"} answered{" "}
          <BeatCount value={answered} /> asks for you.
        </p>
      </div>

      {live.length === 0 ? (
        <p className="text-muted-foreground border-border rounded-lg border border-dashed px-3 py-4 text-sm">
          No rules left. Every permission ask goes to your inbox until you answer one with Always allow.
        </p>
      ) : (
        <>
          {/* overflow-hidden clips the rows while they slide up into their new places.
              Without it a row still sitting in its old spot spills past the shortened
              table and lands on top of the revoked strip. */}
          <motion.div layout="position" transition={layoutSpring} className="overflow-hidden max-md:hidden">
            <Table>
              <TableHeader>
                <TableRow className="hover:bg-transparent">
                  <TableHead>Rule</TableHead>
                  <TableHead>Matches</TableHead>
                  <TableHead>Scope</TableHead>
                  <TableHead className="px-1 text-right">Used</TableHead>
                  <TableHead className="w-0 text-right">Action</TableHead>
                </TableRow>
              </TableHeader>
              {/* A <tr> cannot collapse its height without overlapping what follows it, so
                  a revoked row leaves the table at once and the rows under it slide up.
                  The row itself leaves with the row variant on the phone list below,
                  where the markup is a real list and the collapse is honest. */}
              <TableBody>
                <AnimatePresence initial={false}>
                  {live.map((r) => (
                    <MotionRow key={r.id} layout="position" transition={layoutSpring} variants={row} initial="hidden" animate="show">
                      <TableCell className="py-2.5">
                        <p className="text-sm font-medium">{r.label}</p>
                        <p className="text-muted-foreground text-xs">{r.from}</p>
                      </TableCell>
                      <TableCell className="font-mono text-xs [overflow-wrap:anywhere]">{r.pattern}</TableCell>
                      <TableCell>
                        <Badge variant="outline" className="font-mono font-normal">{r.scope}</Badge>
                      </TableCell>
                      <TableCell className="tnum px-1 text-right font-mono text-sm">{r.uses}</TableCell>
                      <TableCell className="text-right">
                        <RevokeButton rule={r} onConfirm={() => revoke(r.id)} />
                      </TableCell>
                    </MotionRow>
                  ))}
                </AnimatePresence>
              </TableBody>
            </Table>
          </motion.div>

          <motion.div layout="position" transition={layoutSpring} className="md:hidden">
            <ItemGroup className="gap-1">
              <AnimatePresence initial={false}>
                {live.map((r) => (
                  <motion.div key={r.id} layout="position" transition={layoutSpring} variants={row} initial="hidden" animate="show" exit="exit" className="overflow-hidden">
                    <Item variant="outline" className="items-start">
                      <ItemContent className="min-w-0 gap-1.5">
                        <ItemTitle>{r.label}</ItemTitle>
                        <ItemDescription className="font-mono [overflow-wrap:anywhere]">{r.pattern}</ItemDescription>
                        <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
                          <Badge variant="outline" className="font-mono font-normal">{r.scope}</Badge>
                          <span className="text-muted-foreground text-xs">
                            Used <span className="tnum font-mono">{r.uses}</span> times since {day(r.createdAt)}
                          </span>
                        </div>
                        <RevokeButton rule={r} onConfirm={() => revoke(r.id)} className="mt-1 w-fit" />
                      </ItemContent>
                    </Item>
                  </motion.div>
                ))}
              </AnimatePresence>
            </ItemGroup>
          </motion.div>
        </>
      )}

      <AnimatePresence initial={false}>
        {dead.length > 0 && <RevokedStrip key="revoked" rules={dead} onUndo={undo} />}
      </AnimatePresence>

      <p className="text-muted-foreground text-sm">
        A new rule starts in the inbox: answer an ask with Always allow and pick how wide it reaches.
      </p>
    </motion.div>
  )
}
