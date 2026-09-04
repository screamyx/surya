// Screen: result. Feature 5 - the finished job in plain words, with a Ship button
// that walks the PR from open to merged.
import { useState } from "react"
import { motion } from "motion/react"
import { Check, CircleCheck, ExternalLink, GitMerge } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Item, ItemActions, ItemContent, ItemTitle } from "@/components/ui/item"
import { Separator } from "@/components/ui/separator"
import { StatusDot } from "@/components/status"
import { agentById, result } from "@/data"
import { pop, rise, stagger } from "@/motion"
import { cn } from "@/lib/utils"
import { hunks } from "@/screens/result/hunks"

function DiffLine({ line }: { line: string }) {
  const added = line.startsWith("+")
  const removed = line.startsWith("-")
  const meta = line.startsWith("@@")
  return (
    <div
      className={cn(
        "border-l-2 border-transparent px-3 py-0.5 whitespace-pre-wrap",
        added && "bg-accent text-accent-foreground border-primary",
        removed && "bg-muted text-muted-foreground border-border line-through",
        meta && "text-muted-foreground bg-muted/50"
      )}
    >
      {line === "" ? " " : line}
    </div>
  )
}

function DiffView({ path }: { path: string }) {
  const text = hunks[path] ?? ""
  return (
    <Card size="sm" className="gap-0 py-0">
      <div className="text-muted-foreground border-b px-3 py-2 font-mono text-xs break-all">{path}</div>
      <div className="overflow-x-auto font-mono text-xs leading-relaxed">
        {text.split("\n").map((line, i) => (
          <DiffLine key={i} line={line} />
        ))}
      </div>
    </Card>
  )
}

function Stepper() {
  return (
    <ol className="flex flex-col">
      {result.steps.map((s, i) => {
        const last = i === result.steps.length - 1
        const later = s.state === "later"
        return (
          <li key={s.name} className="flex gap-3">
            <div className="flex flex-col items-center">
              <span
                className={cn(
                  "flex size-5 shrink-0 items-center justify-center rounded-full",
                  s.state === "done" && "bg-primary text-primary-foreground",
                  s.state === "ready" && "text-primary ring-primary bg-background ring-2",
                  later && "bg-muted text-muted-foreground"
                )}
              >
                {s.state === "done" ? <Check className="size-3" /> : <span className="size-1.5 rounded-full bg-current" />}
              </span>
              {!last && <span className="bg-border w-px flex-1" />}
            </div>
            <div className={cn("pb-4", last && "pb-0")}>
              <p className={cn("text-sm font-medium", later && "text-muted-foreground")}>{s.name}</p>
              {"note" in s && s.note && <p className="text-muted-foreground text-xs">{s.note}</p>}
            </div>
          </li>
        )
      })}
    </ol>
  )
}

function ShipCard() {
  return (
    <motion.div variants={pop} initial="hidden" animate="show">
      <Card>
        <CardHeader>
          <CardTitle>Ship it</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <Stepper />
          <Separator />
          <ul className="flex flex-col gap-1.5">
            {result.checks.map((c) => (
              <li key={c.name} className="flex items-center gap-2 text-sm">
                <CircleCheck className="text-primary size-4 shrink-0" />
                <span className="truncate">{c.name}</span>
                <span className="text-muted-foreground ml-auto text-xs">{c.state}</span>
              </li>
            ))}
          </ul>
          <div className="flex flex-col gap-2">
            <Button className="h-10 w-full lg:h-9">
              <GitMerge data-icon="inline-start" />
              Merge to v2
            </Button>
            <Button
              variant="secondary"
              className="h-10 w-full lg:h-9"
              nativeButton={false}
              render={<a href={result.pr.url} target="_blank" rel="noreferrer" />}
            >
              <ExternalLink data-icon="inline-start" />
              Open PR
            </Button>
          </div>
          <p className="text-muted-foreground text-xs">Deploy to prod happens on 2026-11-01.</p>
        </CardContent>
      </Card>
    </motion.div>
  )
}

export function ResultScreen() {
  const agent = agentById(result.agentId)
  const [file, setFile] = useState(result.files[0].path)

  return (
    <div className="flex min-w-0 flex-col gap-4 p-4">
      <div className="flex flex-col gap-2">
        <h1 className="u-display max-w-[22ch] text-2xl md:text-3xl">{result.title}</h1>
        <div className="flex flex-wrap items-center gap-1.5">
          <Badge variant="outline" className="gap-1.5">
            <StatusDot status={agent.status} />
            {agent.name}
          </Badge>
          <Badge variant="secondary">PR #{result.pr.number}</Badge>
          <Badge variant="secondary" className="gap-1">
            <Check />
            CI {result.pr.ci}
          </Badge>
          <Badge variant="secondary">
            {result.pr.reviews} review{result.pr.reviews === 1 ? "" : "s"}
          </Badge>
        </div>
      </div>

      <div className="flex flex-col gap-4 lg:grid lg:grid-cols-[minmax(0,1fr)_22rem] lg:items-start">
        <div className="order-1 lg:sticky lg:top-16 lg:order-none lg:col-start-2 lg:row-start-1 lg:row-span-2">
          <ShipCard />
        </div>

        <Card className="order-2 lg:order-none lg:col-start-1 lg:row-start-1">
          <CardHeader>
            <CardTitle>What changed</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-base leading-relaxed">{result.summary}</p>
          </CardContent>
        </Card>

        <div className="order-3 flex min-w-0 flex-col gap-3 lg:order-none lg:col-start-1 lg:row-start-2">
          <div className="flex items-baseline justify-between">
            <h2 className="text-sm font-medium">Files changed</h2>
            <span className="text-muted-foreground text-xs">{result.files.length} files</span>
          </div>
          <motion.div variants={stagger} initial="hidden" animate="show" className="flex flex-col gap-2">
            {result.files.map((f) => (
              <motion.div key={f.path} variants={rise}>
                <Item
                  variant="outline"
                  size="sm"
                  onClick={() => setFile(f.path)}
                  className={cn("cursor-pointer", file === f.path && "bg-muted/50 ring-ring ring-2")}
                >
                  <ItemContent>
                    <ItemTitle className="font-mono text-xs break-all">{f.path}</ItemTitle>
                  </ItemContent>
                  <ItemActions>
                    <Badge variant="secondary">+{f.added}</Badge>
                    <Badge variant="outline">-{f.removed}</Badge>
                  </ItemActions>
                </Item>
              </motion.div>
            ))}
          </motion.div>
          <DiffView path={file} />
        </div>
      </div>
    </div>
  )
}
