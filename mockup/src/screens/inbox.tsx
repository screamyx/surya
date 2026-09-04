// Screen: /inbox - the Needs you queue. The synthesis found eight of ten shipped ADEs
// have no needs-you queue at all (docs/research/synthesis.md), so this is the spine of
// the app: the first waiting item open at full weight, the rest stacked under it.
import { useState } from "react"
import { Link } from "react-router"
import { AnimatePresence, motion } from "motion/react"
import { CheckCheck, ShieldCheck } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { inbox, settings, type InboxItem } from "@/data"
import { swap } from "@/motion"
import { BeatCount } from "@/screens/beat"
import { FilterStrip } from "@/screens/inbox/filter-strip"
import { InboxStack, waiting, type Settled } from "@/screens/inbox/stack"

type Tab = "all" | "needs" | "failed" | "pins" | "results"

// A stopped agent counts as needing you: it rolls up the same way a question does.
const tabFilter: Record<Tab, (i: InboxItem) => boolean> = {
  all: () => true,
  needs: waiting,
  failed: (i) => i.kind === "failed",
  pins: (i) => i.kind === "pin",
  results: (i) => i.kind === "result",
}

const tabLabel: Record<Tab, string> = { all: "All", needs: "Needs you", failed: "Stopped", pins: "Pins", results: "Results" }
const emptyLine: Record<Tab, string> = {
  all: "Nothing waiting. Your agents will ping you here.",
  needs: "No questions right now. Both agents are working.",
  failed: "Nothing stopped. Every agent is still running.",
  pins: "No pins yet. Tap the preview to leave one.",
  results: "No finished work yet. It shows up here when a PR is ready.",
}

export function InboxScreen() {
  const [tab, setTab] = useState<Tab>("all")
  const [push, setPush] = useState(settings.notifications.push)
  // An answered item is still in the inbox, so the filter counts do not move. What moves
  // is how many are still waiting on you, and this line has to say the true number.
  const [settled, setSettled] = useState<Settled[]>([])
  const items = inbox.filter(tabFilter[tab])
  const needs = inbox.filter((i) => waiting(i) && !settled.some((s) => s.id === i.id)).length

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col px-4 py-5 md:px-6 md:py-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <p className="u-overline text-muted-foreground">Inbox</p>
          <h1 className="u-display mt-1 text-3xl md:text-4xl">Needs you</h1>
          <p className="text-muted-foreground mt-1 text-sm">
            {needs > 0 ? (
              <><BeatCount value={needs} /> waiting on you, out of <span className="tnum">{inbox.length}</span> in the inbox.</>
            ) : (
              <>Nothing waiting on you, out of <span className="tnum">{inbox.length}</span> in the inbox.</>
            )}
          </p>
        </div>
        <div className="flex items-start gap-3">
          <div className="text-right">
            <Label htmlFor="push" className="justify-end text-sm">Push notifications</Label>
            <p className="text-muted-foreground text-xs">
              Quiet {settings.notifications.quietFrom} to {settings.notifications.quietTo}
            </p>
          </div>
          <Switch id="push" checked={push} onCheckedChange={setPush} className="mt-1" />
        </div>
      </div>

      <div className="mt-5">
        <FilterStrip
          value={tab}
          onValueChange={setTab}
          tabs={(Object.keys(tabLabel) as Tab[]).map((t) => ({
            id: t,
            label: tabLabel[t],
            count: inbox.filter(tabFilter[t]).length,
            hot: (t === "needs" || t === "failed") && inbox.filter(tabFilter[t]).length > 0,
          }))}
        />
      </div>

      {/* One filter's list leaves before the next arrives, so nothing crosses over the
          top of anything. mode="wait" is what keeps the two lists from overlapping. */}
      <AnimatePresence mode="wait" initial={false}>
        <motion.div key={tab} variants={swap} initial="hidden" animate="show" exit="exit" className="mt-5">
          {items.length === 0 ? (
            <Empty className="min-h-[55svh] border">
              <EmptyHeader>
                <EmptyMedia variant="icon"><CheckCheck /></EmptyMedia>
                <EmptyTitle>Nothing needs attention</EmptyTitle>
                <EmptyDescription>{emptyLine[tab]}</EmptyDescription>
              </EmptyHeader>
              <EmptyContent>
                <Button variant="outline" nativeButton={false} render={<Link to="/new" />}>Start an agent</Button>
              </EmptyContent>
            </Empty>
          ) : (
            <InboxStack items={items} settled={settled} onSettle={setSettled} />
          )}
        </motion.div>
      </AnimatePresence>

      <div className="mt-8">
        <div className="u-seam" />
        <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1 pt-3">
          <p className="text-muted-foreground text-sm">
            {items.length === 0 ? (
              `Nothing in ${tabLabel[tab]}. Switch a filter above for the rest of the inbox.`
            ) : (
              <>
                End of the inbox. <BeatCount value={items.length} />{" "}
                {items.length === 1 ? "item" : "items"} shown
                {tab === "all" ? "" : ` in ${tabLabel[tab]}`}, the ones waiting on you first.
              </>
            )}
          </p>
          <Button variant="ghost" className="text-muted-foreground hover:text-foreground h-8 px-2" nativeButton={false} render={<Link to="/settings" />}>
            <ShieldCheck data-icon="inline-start" />
            Approval rules
          </Button>
        </div>
      </div>
    </div>
  )
}
