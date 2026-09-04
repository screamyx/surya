// Screen: /inbox - feature 2, the Needs You inbox with push.
// One column of cards. Every permission ask, question, pin, stopped agent and result lands here.
import { useState } from "react"
import { motion } from "motion/react"
import { Check } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { inbox, settings, type InboxItem } from "@/data"
import { rise, stagger } from "@/motion"
import { InboxCard } from "@/screens/inbox/cards"

type Tab = "all" | "needs" | "failed" | "pins" | "results"

// A stopped agent counts as needing you: it rolls up the same way a question does.
const tabFilter: Record<Tab, (i: InboxItem) => boolean> = {
  all: () => true,
  needs: (i) => i.kind === "permission" || i.kind === "question" || i.kind === "failed",
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
  const items = inbox.filter(tabFilter[tab])
  const needs = inbox.filter(tabFilter.needs).length

  return (
    <div className="mx-auto w-full max-w-3xl px-4 py-5 md:px-6 md:py-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h1 className="u-display text-3xl md:text-4xl">Needs you</h1>
          <p className="text-muted-foreground text-sm">
            {inbox.length} in your inbox, {needs} waiting on you.
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

      <Tabs value={tab} onValueChange={(v) => setTab(v as Tab)} className="mt-5">
        {/* Five tabs do not fit a 390px phone, so the bar scrolls. justify-start keeps the first
            tab reachable: centring a flex row that overflows clips both ends. */}
        <TabsList className="w-full justify-start overflow-x-auto">
          {(Object.keys(tabLabel) as Tab[]).map((t) => {
            const n = inbox.filter(tabFilter[t]).length
            const hot = (t === "needs" || t === "failed") && n > 0
            return (
              <TabsTrigger key={t} value={t}>
                {tabLabel[t]}
                <Badge variant={hot ? "destructive" : "secondary"} className="px-1.5 text-xs">{n}</Badge>
              </TabsTrigger>
            )
          })}
        </TabsList>
      </Tabs>

      {items.length === 0 ? (
        <Empty className="mt-6 min-h-60 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><Check /></EmptyMedia>
            <EmptyTitle>All clear</EmptyTitle>
            <EmptyDescription>{emptyLine[tab]}</EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <motion.div key={tab} variants={stagger} initial="hidden" animate="show" className="mt-5 flex flex-col gap-4">
          {items.map((item) => (
            <motion.div key={item.id} variants={rise}>
              <InboxCard item={item} />
            </motion.div>
          ))}
        </motion.div>
      )}
    </div>
  )
}
