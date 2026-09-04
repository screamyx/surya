// The second page in the Browser engine: an invented public documentation site on a
// reserved .example domain. It is here because the address bar has to go somewhere, and a
// spec page is the other thing a person asks an agent to read.
//
// It reads as a third application again: a left contents column, a measure-limited article,
// and a code block. Nothing about it looks like surya or like the staff app.
import { Search } from "lucide-react"
import { docsFields, docsNav } from "@/screens/session/preview/browser-data"
import { cn } from "@/lib/utils"

const sample = [
  { k: "stock_no", v: '"KSS-0412"' },
  { k: "title", v: '"2021 Toyota Alphard 2.5 SC"' },
  { k: "price_myr", v: "268800", num: true },
]

function Sample() {
  return (
    <pre data-el="pre.sample" className="bg-foreground text-background/90 shrink-0 overflow-hidden rounded-lg px-2.5 py-2 font-mono text-2xs leading-relaxed">
      <span className="text-code-com">{"// POST /v1/vehicles"}</span>
      {"\n{"}
      {sample.map((l) => (
        <span key={l.k}>
          {"\n  "}
          <span className="text-code-key">"{l.k}"</span>
          {": "}
          <span className={l.num ? "text-code-num" : "text-code-str"}>{l.v}</span>
          {","}
        </span>
      ))}
      {"\n}"}
    </pre>
  )
}

export function SiteDocs({ narrow = false }: { narrow?: boolean }) {
  return (
    <div className="bg-background text-foreground flex size-full min-w-0 flex-col overflow-hidden">
      <header className="flex shrink-0 items-center gap-2 border-b px-2.5 py-1.5">
        <span className="border-border-strong flex size-5 shrink-0 items-center justify-center rounded-sm border font-mono text-2xs font-bold">
          S
        </span>
        <p className="truncate text-xs font-semibold">Stockfeed docs</p>
        <span className="text-muted-foreground ml-auto flex min-w-0 shrink items-center gap-1.5 rounded-md border px-1.5 py-0.5 text-2xs">
          <Search className="size-3 shrink-0" />
          <span className="truncate">Search the reference</span>
        </span>
      </header>

      <div className="flex min-h-0 flex-1 overflow-hidden">
        {!narrow && (
          <nav data-el="nav.reference" className="w-36 shrink-0 border-r px-2 py-2.5">
            <p className="u-overline text-muted-foreground px-1.5 pb-1.5">Reference</p>
            <ul className="flex flex-col gap-0.5">
              {docsNav.map((n) => (
                <li key={n.label}>
                  <span
                    className={cn(
                      "block truncate rounded-md px-1.5 py-1 text-2xs",
                      n.active ? "bg-muted text-foreground font-medium" : "text-muted-foreground",
                    )}
                  >
                    {n.label}
                  </span>
                </li>
              ))}
            </ul>
          </nav>
        )}

        <article className="flex min-h-0 min-w-0 flex-1 flex-col gap-2 px-3 py-2.5">
          <div className="shrink-0">
            <p className="u-overline text-muted-foreground">Vehicle feed</p>
            <h1 data-el="article h1" className="u-display text-lg leading-tight">Post a vehicle</h1>
            <p className="text-muted-foreground max-w-prose text-2xs leading-relaxed">
              One call adds a car to every listing surface. Send the whole record each time. Fields you
              leave out are cleared, not kept.
            </p>
          </div>

          <Sample />

          <div className="min-h-0 min-w-0 flex-1 overflow-hidden">
            <p className="pb-1 text-2xs font-medium">Fields</p>
            <dl className="divide-y border-t">
              {docsFields.map((f) => (
                <div
                  key={f.name}
                  data-el={`.field[data-name="${f.name}"]`}
                  className="flex min-w-0 items-baseline gap-2 py-1"
                >
                  <dt className="w-20 shrink-0 truncate font-mono text-2xs">{f.name}</dt>
                  <span className="text-muted-foreground w-14 shrink-0 truncate font-mono text-2xs">
                    {f.type}
                  </span>
                  <dd className="text-muted-foreground min-w-0 flex-1 truncate text-2xs">{f.note}</dd>
                </div>
              ))}
            </dl>
          </div>
        </article>
      </div>
    </div>
  )
}
