// A2UI card renderer. The agent sends a declarative card, surya draws it from the
// shadcn catalog (decision 4). One switch, one component per card type.
import { Link } from "react-router"
import { CalendarDays, CheckCheck, ExternalLink, GitPullRequest, TrendingUp } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Progress } from "@/components/ui/progress"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import type { CardSample } from "@/data"

function VehicleCard({ card }: { card: Extract<CardSample, { type: "vehicle" }> }) {
  return (
    <Card size="sm" className="overflow-hidden">
      <img src={card.photo} alt={card.title} className="h-40 w-full object-cover sm:h-48" />
      <CardHeader>
        <CardTitle className="leading-snug">{card.title}</CardTitle>
        <CardDescription className="flex flex-wrap items-center gap-1.5">
          <Badge variant="outline" className="font-mono">{card.stockNo}</Badge>
          <Badge variant="secondary">{card.status}</Badge>
        </CardDescription>
        <CardAction className="text-right">
          <div className="font-heading text-base font-medium">{card.price}</div>
        </CardAction>
      </CardHeader>
      <CardContent className="space-y-1.5">
        <div className="text-muted-foreground flex items-center justify-between text-xs">
          <span className="flex items-center gap-1.5"><CalendarDays className="size-3.5" />{card.days} days on the lot</span>
          <span className="tabular-nums">of 90</span>
        </div>
        <Progress value={Math.round((card.days / 90) * 100)} />
      </CardContent>
      <CardFooter className="gap-2">
        <Button size="sm">Set follow-ups</Button>
        <Button size="sm" variant="outline">Open the listing<ExternalLink data-icon="inline-end" /></Button>
      </CardFooter>
    </Card>
  )
}

function TableCard({ card }: { card: Extract<CardSample, { type: "table" }> }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{card.title}</CardTitle>
        <CardDescription>{card.rows.length} leads, oldest first</CardDescription>
      </CardHeader>
      <CardContent className="px-0">
        <Table>
          <TableHeader>
            <TableRow>
              {card.columns.map((c) => (
                <TableHead key={c} className="first:pl-4 last:pr-4">{c}</TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {card.rows.map((row) => (
              <TableRow key={row[0]}>
                {row.map((cell, i) => (
                  <TableCell key={cell} className={i === 0 ? "pl-4 font-medium" : "text-muted-foreground last:pr-4"}>{cell}</TableCell>
                ))}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
      <CardFooter className="gap-2">
        <Button size="sm">Set follow-ups</Button>
        <Button size="sm" variant="ghost">Show all leads</Button>
      </CardFooter>
    </Card>
  )
}

function FormCard({ card }: { card: Extract<CardSample, { type: "form" }> }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{card.title}</CardTitle>
        <CardDescription>Check the details, then save.</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3 sm:grid-cols-2">
        {card.fields.map((f) => (
          <div key={f.label} className="grid gap-1.5">
            <span className="text-muted-foreground text-xs">{f.label}</span>
            {f.type === "select" ? (
              <Select defaultValue={f.value}>
                <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  {(f.options ?? []).map((o) => <SelectItem key={o} value={o}>{o}</SelectItem>)}
                </SelectContent>
              </Select>
            ) : (
              <Input type={f.type === "date" ? "date" : "text"} defaultValue={f.value} />
            )}
          </div>
        ))}
      </CardContent>
      <CardFooter className="gap-2">
        <Button size="sm">{card.submit}</Button>
        <Button size="sm" variant="ghost">Cancel</Button>
      </CardFooter>
    </Card>
  )
}

function ApprovalCard({ card }: { card: Extract<CardSample, { type: "approval" }> }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{card.title}</CardTitle>
        <CardDescription>{card.summary}</CardDescription>
      </CardHeader>
      <CardContent>
        <pre className="bg-muted overflow-x-auto rounded-md p-3 font-mono text-xs">{card.code}</pre>
      </CardContent>
      <CardFooter className="gap-2 max-sm:flex-col max-sm:items-stretch">
        <Button size="sm">Approve<CheckCheck data-icon="inline-end" /></Button>
        <Button size="sm" variant="outline">Change it first</Button>
      </CardFooter>
    </Card>
  )
}

function DiffSummaryCard({ card }: { card: Extract<CardSample, { type: "diff-summary" }> }) {
  const added = card.files.reduce((n, f) => n + f.added, 0)
  const removed = card.files.reduce((n, f) => n + f.removed, 0)
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{card.title}</CardTitle>
        <CardDescription>{card.note}</CardDescription>
        <CardAction className="flex items-center gap-1.5">
          <Badge variant="secondary">+{added}</Badge>
          <Badge variant="outline">-{removed}</Badge>
        </CardAction>
      </CardHeader>
      <CardContent className="grid gap-1.5">
        {card.files.map((f) => (
          <div key={f.path} className="flex items-center gap-2">
            <span className="text-muted-foreground min-w-0 flex-1 truncate font-mono text-xs" >{f.path}</span>
            <Badge variant="secondary">+{f.added}</Badge>
            <Badge variant="outline">-{f.removed}</Badge>
          </div>
        ))}
      </CardContent>
      <CardFooter className="gap-2">
        <Button size="sm">Open PR<GitPullRequest data-icon="inline-end" /></Button>
        <Button size="sm" variant="ghost" nativeButton={false} render={<Link to="/w/project-jag/files" />}>Open in editor</Button>
      </CardFooter>
    </Card>
  )
}

function MetricCard({ card }: { card: Extract<CardSample, { type: "metric" }> }) {
  const max = Math.max(...card.series, 1)
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{card.title}</CardTitle>
        <CardDescription className="flex items-center gap-1.5">
          <TrendingUp className="size-3.5" />{card.delta}
        </CardDescription>
        <CardAction className="font-heading text-2xl leading-none font-medium tabular-nums">{card.value}</CardAction>
      </CardHeader>
      <CardContent>
        <div className="flex h-12 items-end gap-1.5">
          {card.series.map((v, i) => (
            <div key={i} className="bg-primary min-h-1 flex-1 rounded-sm" style={{ height: `${(v / max) * 100}%` }} />
          ))}
        </div>
        <div className="text-muted-foreground mt-1.5 flex justify-between text-xs">
          <span>Mon</span><span>Sun</span>
        </div>
      </CardContent>
      <CardFooter className="gap-2">
        <Button size="sm" variant="outline">Break it down by salesperson</Button>
      </CardFooter>
    </Card>
  )
}

export function A2UICard({ card }: { card: CardSample }) {
  switch (card.type) {
    case "vehicle": return <VehicleCard card={card} />
    case "table": return <TableCard card={card} />
    case "form": return <FormCard card={card} />
    case "approval": return <ApprovalCard card={card} />
    case "diff-summary": return <DiffSummaryCard card={card} />
    case "metric": return <MetricCard card={card} />
  }
}
