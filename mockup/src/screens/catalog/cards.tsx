// The A2UI catalog renderer. One card type per branch, all built from stock shadcn,
// so an agent answer and a surya panel are the same design system.
import { useId } from "react"
import { Copy, ExternalLink, Share2, TrendingUp } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import type { CardSample } from "@/data"

// The generic record card: photo, title, id, one big value, a few labelled fields,
// and up to three actions. Any workspace fills it with whatever it keeps records of.
function RecordCard({ card }: { card: Extract<CardSample, { photo: string }> }) {
  const fields = [
    { label: "Status", value: card.status },
    { label: "Age", value: `${card.days} days` },
  ]
  return (
    <div className="flex flex-col">
      <img src={card.photo} alt={card.title} className="aspect-video w-full rounded-lg border object-cover" />
      <p className="mt-3 text-sm font-medium">{card.title}</p>
      <p className="text-muted-foreground font-mono text-xs">{card.stockNo}</p>
      <p className="mt-2 text-2xl font-semibold tracking-tight">{card.price}</p>
      <dl className="mt-3 flex flex-col gap-1.5">
        {fields.map((f) => (
          <div key={f.label} className="flex items-baseline justify-between gap-4">
            <dt className="text-muted-foreground text-sm">{f.label}</dt>
            <dd className="truncate text-sm font-medium">{f.value}</dd>
          </div>
        ))}
      </dl>
      <div className="mt-4 flex flex-wrap gap-2">
        <Button size="sm">Open<ExternalLink data-icon="inline-end" /></Button>
        <Button size="sm" variant="outline"><Copy data-icon="inline-start" />Copy id</Button>
        <Button size="sm" variant="outline"><Share2 data-icon="inline-start" />Share</Button>
      </div>
    </div>
  )
}

function TableCard({ card }: { card: Extract<CardSample, { type: "table" }> }) {
  return (
    <div className="flex flex-col">
      <p className="text-sm font-medium">{card.title}</p>
      <div className="mt-3 -mx-1 overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow>
              {card.columns.map((c) => <TableHead key={c} className="whitespace-nowrap">{c}</TableHead>)}
            </TableRow>
          </TableHeader>
          <TableBody>
            {card.rows.map((row) => (
              <TableRow key={row[0]}>
                {row.map((cell, i) => <TableCell key={card.columns[i]} className={i === 0 ? "font-medium whitespace-nowrap" : "text-muted-foreground whitespace-nowrap"}>{cell}</TableCell>)}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
      <Separator className="my-3" />
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="text-muted-foreground text-xs">{card.rows.length} rows</span>
        <Button size="sm" variant="outline">Set follow-ups for all</Button>
      </div>
    </div>
  )
}

function FormCard({ card }: { card: Extract<CardSample, { type: "form" }> }) {
  // Two form cards can sit on the same page, so the field ids have to be unique per card.
  const uid = useId()
  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm font-medium">{card.title}</p>
      {card.fields.map((f) => (
        <div key={f.label} className="grid gap-1.5">
          <Label htmlFor={`${uid}-${f.label}`}>{f.label}</Label>
          {f.type === "select" ? (
            <Select defaultValue={f.value}>
              <SelectTrigger id={`${uid}-${f.label}`} className="w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                {(f.options ?? []).map((o) => <SelectItem key={o} value={o}>{o}</SelectItem>)}
              </SelectContent>
            </Select>
          ) : (
            <Input id={`${uid}-${f.label}`} type={f.type === "date" ? "date" : "text"} defaultValue={f.value} />
          )}
        </div>
      ))}
      <Button className="mt-1 w-full sm:w-fit">{card.submit}</Button>
    </div>
  )
}

function ApprovalCard({ card }: { card: Extract<CardSample, { type: "approval" }> }) {
  return (
    <div className="flex flex-col">
      <p className="text-sm font-medium">{card.title}</p>
      <p className="text-muted-foreground mt-1 text-sm">{card.summary}</p>
      <pre className="bg-muted text-muted-foreground mt-3 rounded-lg p-3 font-mono text-xs leading-relaxed break-words whitespace-pre-wrap"><code>{card.code}</code></pre>
      <div className="mt-4 flex flex-wrap gap-2">
        <Button size="sm">Approve</Button>
        <Button size="sm" variant="outline">Reject</Button>
      </div>
    </div>
  )
}

function DiffSummaryCard({ card }: { card: Extract<CardSample, { type: "diff-summary" }> }) {
  return (
    <div className="flex flex-col">
      <p className="text-sm font-medium">{card.title}</p>
      <ul className="mt-3 flex flex-col gap-2">
        {card.files.map((f) => (
          <li key={f.path} className="flex items-center gap-2">
            <span className="min-w-0 flex-1 truncate font-mono text-xs" title={f.path}>{f.path}</span>
            <Badge variant="secondary">+{f.added}</Badge>
            {f.removed > 0 && <Badge variant="outline">-{f.removed}</Badge>}
          </li>
        ))}
      </ul>
      <p className="text-muted-foreground mt-3 text-sm">{card.note}</p>
      <div className="mt-4">
        <Button size="sm" variant="outline">Open diff</Button>
      </div>
    </div>
  )
}

function MetricCard({ card }: { card: Extract<CardSample, { type: "metric" }> }) {
  const peak = Math.max(...card.series)
  return (
    <div className="flex flex-col">
      <p className="text-muted-foreground text-sm">{card.title}</p>
      <p className="mt-1 text-4xl font-semibold tracking-tight tabular-nums">{card.value}</p>
      <p className="text-muted-foreground mt-1 flex items-center gap-1.5 text-sm">
        <TrendingUp className="size-3.5" />
        {card.delta}
      </p>
      <div className="mt-4 flex h-16 items-end gap-1.5" aria-hidden>
        {card.series.map((v, i) => (
          <div key={i} className="bg-primary min-h-1 w-2 rounded-sm" style={{ height: `${(v / peak) * 100}%` }} />
        ))}
      </div>
    </div>
  )
}

export function A2UICard({ card }: { card: CardSample }) {
  switch (card.type) {
    case "table": return <TableCard card={card} />
    case "form": return <FormCard card={card} />
    case "approval": return <ApprovalCard card={card} />
    case "diff-summary": return <DiffSummaryCard card={card} />
    case "metric": return <MetricCard card={card} />
    default: return <RecordCard card={card} />
  }
}
