// The page inside the Browser engine. An invented ads manager on a reserved .example
// domain: no real brand, no real logo, no real product name. It reads as a different
// application than surya because it wears a dark rail and a dense number table, the way
// fake-dms.tsx reads as a different application through density and a bottom tab bar.
//
// This is the register decision 21 asks for: something a person would plausibly ask an
// agent to read or operate, that no proxy-and-iframe engine could ever show.
import { BarChart3, Bell, ChevronDown, Layers, Megaphone, Search, Settings, Users } from "lucide-react"
import { adsTotals, campaigns, type Campaign } from "@/screens/session/preview/browser-data"
import { cn } from "@/lib/utils"

const rail = [
  { icon: BarChart3, label: "Overview", active: false },
  { icon: Megaphone, label: "Campaigns", active: true },
  { icon: Users, label: "Audiences", active: false },
  { icon: Layers, label: "Creative", active: false },
  { icon: Settings, label: "Settings", active: false },
]

const statusTone: Record<Campaign["status"], string> = {
  Active: "text-ok",
  Paused: "text-muted-foreground",
  "In review": "text-warn",
}

function StatusDot({ status, name }: { status: Campaign["status"]; name: string }) {
  return (
    <span
      data-el={`.campaign-row[data-name="${name}"] .status`}
      className={cn("status inline-flex items-center gap-1.5 whitespace-nowrap", statusTone[status])}
    >
      <span className="size-1.5 shrink-0 rounded-full bg-current" />
      {status}
    </span>
  )
}

function Totals() {
  return (
    <div className="totals grid grid-cols-3 gap-1.5">
      {adsTotals.map((t, i) => (
        <div
          key={t.label}
          data-el={`.totals .${t.key}`}
          className={cn(
            "bg-card flex min-w-0 flex-col gap-0.5 rounded-lg border px-2 py-1.5",
            t.key,
            // One tile leads: the spend is the number the dealer opens this page for.
            i === 0 && "border-border-strong",
          )}
        >
          <p className="text-muted-foreground truncate text-2xs">{t.label}</p>
          <p className={cn("tnum truncate font-semibold", i === 0 ? "text-base" : "text-sm")}>{t.value}</p>
          <p className="text-muted-foreground truncate text-2xs">{t.note}</p>
        </div>
      ))}
    </div>
  )
}

function Row({ c, narrow }: { c: Campaign; narrow: boolean }) {
  return (
    <tr
      className="campaign-row border-b last:border-0"
      data-name={c.name}
      data-el={`.campaign-row[data-name="${c.name}"]`}
    >
      <td className="max-w-0 py-1 pr-2 pl-2">
        <p className="truncate text-2xs font-medium">{c.name}</p>
      </td>
      <td className="py-1 pr-2 text-2xs">
        <StatusDot status={c.status} name={c.name} />
      </td>
      <td className="tnum py-1 pr-2 text-right text-2xs whitespace-nowrap">{c.spend}</td>
      {!narrow && <td className="tnum py-1 pr-2 text-right text-2xs">{c.leads}</td>}
      <td className="tnum py-1 text-right text-2xs whitespace-nowrap">{c.cpl}</td>
    </tr>
  )
}

export function SiteAds({ narrow = false }: { narrow?: boolean }) {
  const shown = narrow ? campaigns.slice(0, 4) : campaigns
  return (
    <div className="bg-canvas text-foreground flex size-full min-w-0 overflow-hidden">
      {!narrow && (
        <nav data-el="nav.side-nav" className="bg-foreground text-background flex w-32 shrink-0 flex-col gap-0.5 px-2 py-2.5">
          <p className="u-overline text-background/60 px-1.5 pb-1.5">Anchorpoint</p>
          {rail.map((r) => (
            <span
              key={r.label}
              className={cn(
                "flex items-center gap-2 rounded-md px-1.5 py-1 text-2xs",
                r.active ? "bg-background/15 text-background font-medium" : "text-background/70",
              )}
            >
              <r.icon className="size-3.5 shrink-0" />
              <span className="truncate">{r.label}</span>
            </span>
          ))}
          <span className="mt-auto flex items-center gap-2 px-1.5 pt-2 text-2xs text-background/60">
            <span className="bg-background/20 flex size-5 shrink-0 items-center justify-center rounded-full font-medium">
              ZL
            </span>
            <span className="truncate">KSS Auto</span>
          </span>
        </nav>
      )}

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="bg-card flex shrink-0 items-center gap-2 border-b px-2.5 py-1.5">
          {narrow && (
            <span className="bg-foreground text-background flex size-5 shrink-0 items-center justify-center rounded-md text-2xs font-bold">
              A
            </span>
          )}
          <p className="truncate text-xs font-semibold">Campaigns</p>
          <span
            data-el="header .date-range"
            className="text-muted-foreground flex shrink-0 items-center gap-1 rounded-md border px-1.5 py-0.5 text-2xs"
          >
            Last 7 days
            <ChevronDown className="size-3" />
          </span>
          <div className="text-muted-foreground ml-auto flex shrink-0 items-center gap-2">
            <Search className="size-3.5" />
            <Bell className="size-3.5" />
          </div>
        </header>

        <div className="flex min-h-0 flex-1 flex-col gap-2 px-2.5 py-2">
          <Totals />

          <div className="bg-card flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border">
            <table data-el="table.campaigns" className="w-full table-fixed border-collapse">
              <thead>
                <tr className="text-muted-foreground border-b text-left">
                  <th className="px-2 py-1 text-2xs font-normal">Campaign</th>
                  <th className="py-1 pr-2 text-2xs font-normal">Status</th>
                  <th className="py-1 pr-2 text-right text-2xs font-normal">Spend</th>
                  {!narrow && <th className="py-1 pr-2 text-right text-2xs font-normal">Leads</th>}
                  <th className="py-1 pr-2 text-right text-2xs font-normal">Per lead</th>
                </tr>
              </thead>
              <tbody>
                {shown.map((c) => (
                  <Row key={c.name} c={c} narrow={narrow} />
                ))}
              </tbody>
            </table>
            <p data-el=".campaign-count" className="text-muted-foreground mt-auto border-t px-2 py-1 text-2xs">
              {shown.length} of {campaigns.length} campaigns
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}
