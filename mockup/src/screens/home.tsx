// Screen: / - the attention queue (decision 20, point 1).
// Home used to be an inventory grouped by server, which answers "where things live".
// A person opening surya is asking "what now", so the order is: what needs you, what
// is running, what is finished and unshipped. Workspace is a label, not the grouping.
//
// The queue is the one thing on this page that changes, so it is the one thing that
// moves: answering the lead ask drops it to an answered line and the next ask rises
// into the open slot on a layout spring, rather than the list teleporting.
import { useState } from "react"
import { Link } from "react-router"
import { AnimatePresence, motion } from "motion/react"
import { ArrowRight, Check, Rocket } from "lucide-react"
import { Button } from "@/components/ui/button"
import { StatusDot } from "@/components/status"
import { BeatCount } from "@/screens/beat"
import { LeadAsk, AskRow, type Answer } from "@/screens/home/ask"
import { MachineList, StartBox } from "@/screens/home/aside"
import {
  agents, fmtTime, inbox, servers, settings, workspaces,
  type Agent, type InboxItem,
} from "@/data"
import { layoutSpring, pop, rise, row, stagger } from "@/motion"
import { cn } from "@/lib/utils"

// A stopped agent needs you as much as a question does, so it lands in the same queue.
const needsYou: InboxItem[] = inbox.filter((i) => i.kind === "permission" || i.kind === "question" || i.kind === "failed")
const all = agents
const working = all.filter((a) => a.status === "working")
const done = all.filter((a) => a.status === "done")
const idle = all.filter((a) => a.status === "idle")
const wsName = (id: string) => workspaces.find((w) => w.id === id)?.name ?? id

function Section({ label, count, children }: { label: string; count: number; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <h2 className="u-overline text-muted-foreground flex items-baseline gap-2">
        {label}
        <BeatCount value={count} className="text-muted-foreground" />
      </h2>
      {children}
    </section>
  )
}

// One line per agent. The differentiator is what varies: the summary, and the time.
function AgentLine({ a, trailing }: { a: Agent; trailing?: React.ReactNode }) {
  return (
    <div className="hover:bg-accent group flex min-w-0 items-center gap-3 rounded-lg px-3 py-2 transition-colors">
      <StatusDot status={a.status} />
      <Link to={`/w/${a.workspaceId}/agent/${a.id}`} className="shrink-0 text-sm font-medium">
        {a.name}
      </Link>
      <span className="text-muted-foreground hidden shrink-0 font-mono text-xs sm:inline">{wsName(a.workspaceId)}</span>
      <span className="text-muted-foreground min-w-0 flex-1 truncate text-sm">{a.summary}</span>
      {trailing ?? <span className="text-muted-foreground tnum hidden shrink-0 text-xs sm:inline">{fmtTime(a.lastEventAt)}</span>}
    </div>
  )
}

function QuietQueue() {
  return (
    <motion.div variants={pop} initial="hidden" animate="show" className="bg-card shadow-raise flex flex-col items-start gap-2 rounded-xl px-6 py-10">
      <Check className="text-ok size-6" />
      <p className="text-lg font-medium">Nothing needs attention</p>
      <p className="text-muted-foreground max-w-[46ch] text-sm">
        Every agent is either working or waiting for a task. surya buzzes this phone the moment one of them stops.
      </p>
    </motion.div>
  )
}

// The queue. Open asks first, the answered ones under them, and the whole list on one
// layout spring so a decision reflows the section instead of redrawing it.
function Queue({ answers, onAnswer, onUndo }: {
  answers: Record<string, Answer>
  onAnswer: (id: string, a: Answer) => void
  onUndo: (id: string) => void
}) {
  const open = needsYou.filter((i) => !answers[i.id])
  const settled = needsYou.filter((i) => answers[i.id])
  const answer = onAnswer
  const undo = onUndo

  return (
    <Section label="Needs you" count={open.length}>
      <motion.div layout="position" transition={layoutSpring} className="flex flex-col">
        <AnimatePresence mode="popLayout" initial={false}>
          {open.length === 0 && settled.length === 0 && <QuietQueue key="quiet" />}

          {open[0] && (
            <motion.div key={open[0].id} layout="position" transition={layoutSpring} variants={pop} initial="hidden" animate="show" exit="hidden">
              <LeadAsk item={open[0]} onAnswer={(a) => answer(open[0].id, a)} />
            </motion.div>
          )}
        </AnimatePresence>

        <motion.div layout="position" transition={layoutSpring} className="-mx-3 mt-1 flex flex-col">
          <AnimatePresence initial={false}>
            {open.slice(1).map((i) => (
              <motion.div key={i.id} layout="position" transition={layoutSpring} variants={row} initial="hidden" animate="show" exit="exit">
                <AskRow item={i} />
              </motion.div>
            ))}
            {settled.map((i) => (
              <motion.div key={i.id} layout="position" transition={layoutSpring} variants={row} initial="hidden" animate="show" exit="exit">
                <AskRow item={i} answer={answers[i.id]} onUndo={() => undo(i.id)} />
              </motion.div>
            ))}
          </AnimatePresence>
        </motion.div>

        {open.length === 0 && settled.length > 0 && (
          <motion.p layout="position" transition={layoutSpring} className="text-muted-foreground mt-2 text-sm">
            That is the queue answered. Undo any line above to put it back.
          </motion.p>
        )}
      </motion.div>
    </Section>
  )
}

export function HomeScreen() {
  const [answers, setAnswers] = useState<Record<string, Answer>>({})
  const waiting = needsYou.filter((i) => !answers[i.id]).length

  return (
    <div className="mx-auto w-full max-w-6xl px-4 py-6 md:px-6 md:py-8">
      <header className="flex flex-col gap-1">
        <h1 className="u-display text-3xl md:text-4xl">Good morning, {settings.user.name}</h1>
        <p className="text-muted-foreground text-base">
          {waiting > 0 ? (
            <>
              <BeatCount value={waiting} /> {waiting === 1 ? "thing needs" : "things need"} you.{" "}
              {working.length} agents are still working.
            </>
          ) : (
            `Nothing needs you. ${working.length} agents are still working.`
          )}
        </p>
      </header>

      <motion.div
        variants={stagger}
        initial="hidden"
        animate="show"
        className="mt-6 grid gap-6 lg:grid-cols-[minmax(0,1fr)_20rem] lg:items-start lg:gap-8"
      >
        <div className="flex min-w-0 flex-col gap-7">
          <motion.div variants={rise}>
            <Queue
              answers={answers}
              onAnswer={(id, a) => setAnswers((prev) => ({ ...prev, [id]: a }))}
              onUndo={(id) => setAnswers(({ [id]: _gone, ...rest }) => rest)}
            />
          </motion.div>

          <motion.div variants={rise}>
            <Section label="Working" count={working.length}>
              <div className="-mx-3 flex flex-col">
                {working.map((a) => <AgentLine key={a.id} a={a} />)}
              </div>
            </Section>
          </motion.div>

          <motion.div variants={rise}>
            <Section label="Done, not shipped" count={done.length}>
              <div className="-mx-3 flex flex-col">
                {done.map((a) => (
                  <AgentLine
                    key={a.id}
                    a={a}
                    trailing={
                      <Button variant="outline" size="sm" nativeButton={false} className="shrink-0" render={<Link to={`/w/${a.workspaceId}/result/${a.id}`} />}>
                        <Rocket data-icon="inline-start" />
                        See result
                      </Button>
                    }
                  />
                ))}
              </div>
            </Section>
          </motion.div>

          {idle.length > 0 && (
            <motion.div variants={rise}>
              <Section label="Idle" count={idle.length}>
                <div className="-mx-3 flex flex-col">
                  {idle.map((a) => (
                    <AgentLine
                      key={a.id}
                      a={a}
                      trailing={
                        <Button variant="ghost" size="sm" nativeButton={false} className="text-muted-foreground shrink-0" render={<Link to={`/new?ws=${a.workspaceId}`} />}>
                          Give a task
                          <ArrowRight data-icon="inline-end" />
                        </Button>
                      }
                    />
                  ))}
                </div>
              </Section>
            </motion.div>
          )}
        </div>

        <motion.aside variants={rise} className="flex min-w-0 flex-col gap-4 lg:sticky lg:top-18">
          <StartBox />
          <MachineList />
        </motion.aside>
      </motion.div>

      <footer className={cn("mt-10 flex flex-col gap-3")}>
        <div className="u-seam" />
        <p className="text-muted-foreground text-xs">
          {servers.length} servers, {workspaces.length} workspaces, {all.length} agents.
          {" "}This app was opened from {settings.daemon.host}, which keeps the list.
        </p>
      </footer>
    </div>
  )
}
