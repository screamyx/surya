// The app inside the preview frame. Static stand-in for the project-jag staff app,
// laid out so the pin coordinates in data.ts land on the thing each pin talks about.
import { Bell, Calendar, Car, Check, ChevronRight, Home, MessageCircle, Phone, RefreshCw, Search, Upload, Users } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Separator } from "@/components/ui/separator"
import { cn } from "@/lib/utils"

type Stock = { no: string; title: string; price: string; days: number; badge?: string }

const stock: Stock[] = [
  { no: "KSS-0412", title: "2021 Toyota Alphard 2.5 SC", price: "RM 268,800", days: 34 },
  { no: "KSS-0398", title: "2020 Toyota Vellfire 2.5 ZG", price: "RM 298,000", days: 51, badge: "Reserved" },
  { no: "KSS-0421", title: "2022 Toyota Harrier 2.0 Luxury", price: "RM 232,500", days: 12, badge: "New in" },
  { no: "KSS-0405", title: "2019 Honda CR-V 1.5 TC-P", price: "RM 128,800", days: 63 },
  { no: "KSS-0430", title: "2023 Perodua Alza 1.5 AV", price: "RM 68,500", days: 8, badge: "New in" },
  { no: "KSS-0417", title: "2021 Mercedes-Benz C200 AMG Line", price: "RM 245,000", days: 27 },
]

const photo = (seed: string) => `https://picsum.photos/seed/${seed}/400/260`

function VehicleTile({ v }: { v: Stock }) {
  return (
    <div className="bg-card flex min-h-0 min-w-0 flex-col overflow-hidden rounded-lg border">
      <div className="bg-muted relative min-h-0 flex-1">
        <img src={photo(v.no)} alt={v.title} className="size-full object-cover" />
        {v.badge && (
          <Badge variant="secondary" className="absolute top-1 left-1 h-4 px-1.5 text-[10px] shadow-sm">
            {v.badge}
          </Badge>
        )}
      </div>
      <div className="flex shrink-0 flex-col px-1.5 py-1">
        <p className="truncate text-[11px] leading-tight font-medium">{v.title}</p>
        <p className="text-muted-foreground truncate text-[11px] leading-tight">
          <span className="text-foreground font-semibold">{v.price}</span> · {v.days}d
        </p>
      </div>
    </div>
  )
}

// The tile the first pin points at. It carries the class that pin's selector names.
function UploadTile() {
  return (
    <button
      type="button"
      className="upload-tile bg-muted/40 text-muted-foreground hover:bg-muted flex min-h-0 flex-col items-center justify-center gap-1 rounded-lg border border-dashed px-2 py-1 text-center"
    >
      <Upload className="size-4 shrink-0" />
      <span className="text-foreground text-[11px] leading-tight font-medium">Add photos</span>
      <span className="text-[10px] leading-tight">Uploading 3 of 5</span>
      <span className="bg-muted h-1 w-4/5 shrink-0 overflow-hidden rounded-full">
        <span className="bg-primary block h-full w-3/5 rounded-full" />
      </span>
    </button>
  )
}

function LeadCard() {
  return (
    <div className="lead-card bg-card flex min-w-0 flex-col gap-2 rounded-lg border p-2.5">
      <div className="flex items-center justify-between gap-2">
        <p className="follow-up text-muted-foreground truncate text-[11px]">Next follow-up: not set</p>
        <ChevronRight className="text-muted-foreground size-3.5 shrink-0" />
      </div>
      <Separator />
      <div className="flex min-w-0 items-center gap-2.5">
        <div className="bg-muted text-muted-foreground flex size-8 shrink-0 items-center justify-center rounded-full text-[11px] font-medium">
          AF
        </div>
        <div className="min-w-0 flex-1">
          <p className="truncate text-[13px] leading-tight font-medium">Ahmad Faizal</p>
          <p className="text-muted-foreground truncate text-[11px] leading-tight">
            Walk-in · asked about the Alphard SC · 2 days ago
          </p>
        </div>
        <Badge variant="outline" className="h-5 shrink-0 text-[10px]">
          Zul
        </Badge>
      </div>
      <div className="grid grid-cols-3 gap-1.5">
        {[
          { icon: Phone, label: "Call" },
          { icon: MessageCircle, label: "WhatsApp" },
          { icon: Calendar, label: "Book" },
        ].map((a) => (
          <span
            key={a.label}
            className="bg-muted/60 text-muted-foreground flex items-center justify-center gap-1 rounded-md px-2 py-1 text-[11px]"
          >
            <a.icon className="size-3" />
            {a.label}
          </span>
        ))}
      </div>
    </div>
  )
}

const tabs = [
  { icon: Home, label: "Today", active: true },
  { icon: Car, label: "Stock", active: false },
  { icon: Users, label: "Leads", active: false },
  { icon: Calendar, label: "Diary", active: false },
]

export function FakeDms({ narrow = false, cols = 4 }: { narrow?: boolean; cols?: 2 | 3 | 4 }) {
  const gridCols = cols === 2 ? "grid-cols-2" : cols === 3 ? "grid-cols-3" : "grid-cols-4"
  // Fewer cars on the narrow device, so the grid keeps two rows and the pins stay on target.
  const shown = narrow ? stock.slice(0, 4) : stock
  const head = shown.slice(0, 2)
  const tail = shown.slice(2)

  return (
    <div className="bg-background text-foreground flex size-full flex-col overflow-hidden">
      <header className="flex shrink-0 flex-col gap-2 border-b px-3 py-2">
        <div className="flex h-8 items-center gap-2">
          <div className="bg-primary text-primary-foreground flex size-6 shrink-0 items-center justify-center rounded-md text-[11px] font-bold">
            K
          </div>
          <p className="truncate text-[13px] font-semibold">KSS Auto · Staff</p>
          <div className="text-muted-foreground ml-auto flex shrink-0 items-center gap-2">
            <RefreshCw className="size-3.5" />
            <Bell className="size-3.5" />
            <span className="bg-muted text-muted-foreground flex size-6 items-center justify-center rounded-full text-[10px] font-medium">
              ZL
            </span>
          </div>
        </div>
        <div className="flex h-8 items-center gap-2">
          <div className="bg-muted text-muted-foreground flex h-full min-w-0 flex-1 items-center gap-2 rounded-md px-2 text-[11px]">
            <Search className="size-3.5 shrink-0" />
            <span className="truncate">Search stock, leads, plate number</span>
          </div>
          <Badge className="sync-badge h-6 shrink-0 gap-1" variant="secondary">
            <Check className="size-3" />
            Synced
          </Badge>
          <Badge variant="outline" className="h-6 shrink-0 gap-1">
            <Upload className="size-3" />3 uploading
          </Badge>
        </div>
      </header>

      <div className="flex min-h-0 flex-1 flex-col gap-1.5 px-3 py-2">
        <div className="flex h-5 shrink-0 items-baseline justify-between">
          <p className="text-[13px] font-medium">Stock</p>
          <p className="text-muted-foreground text-[11px]">{shown.length} cars · 2 reserved</p>
        </div>

        <div className={cn("vehicle-grid grid min-h-0 flex-1 auto-rows-fr gap-1.5", gridCols)}>
          {head.map((v) => (
            <VehicleTile key={v.no} v={v} />
          ))}
          <UploadTile />
          {tail.map((v) => (
            <VehicleTile key={v.no} v={v} />
          ))}
          <span className="bg-muted/40 text-muted-foreground flex min-h-0 items-center justify-center gap-1 rounded-lg border text-[11px]">
            See all 42 cars
            <ChevronRight className="size-3" />
          </span>
        </div>

        <div className="flex h-5 shrink-0 items-baseline justify-between pt-1">
          <p className="text-[13px] font-medium">Today's lead</p>
          <p className="text-muted-foreground text-[11px]">1 of 4</p>
        </div>
        <LeadCard />
      </div>

      <nav className="grid shrink-0 grid-cols-4 border-t">
        {tabs.map((t) => (
          <span
            key={t.label}
            className={cn(
              "flex flex-col items-center gap-0.5 py-1.5 text-[10px]",
              t.active ? "text-foreground" : "text-muted-foreground"
            )}
          >
            <t.icon className="size-4" />
            {t.label}
          </span>
        ))}
      </nav>
    </div>
  )
}
