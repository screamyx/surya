// Screen: preview. Feature 4 - the live app with pins. The frame holds a static stand-in
// for the project-jag staff app so every pin lands on something a person recognises.
import { useState } from "react"
import { Link, useParams } from "react-router"
import { motion } from "motion/react"
import { ExternalLink, MapPin, Monitor, RotateCw, Smartphone, Tablet } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Item, ItemContent, ItemDescription, ItemGroup, ItemMedia, ItemTitle } from "@/components/ui/item"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Separator } from "@/components/ui/separator"
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet"
import { Toggle } from "@/components/ui/toggle"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { pins, tasks, workspaceById, type Pin } from "@/data"
import { pop, rise, stagger } from "@/motion"
import { cn } from "@/lib/utils"
import { FakeDms } from "@/screens/preview/fake-dms"

type Device = "phone" | "tablet" | "desktop"

const devices: { value: Device; label: string; icon: typeof Monitor }[] = [
  { value: "phone", label: "Phone", icon: Smartphone },
  { value: "tablet", label: "Tablet", icon: Tablet },
  { value: "desktop", label: "Desktop", icon: Monitor },
]

const frameSize: Record<Device, string> = {
  phone: "w-[390px] max-w-full aspect-[390/720]",
  tablet: "w-[820px] max-w-full aspect-[4/3]",
  desktop: "w-full aspect-[16/10]",
}

const cols: Record<Device, 2 | 3 | 4> = { phone: 2, tablet: 3, desktop: 4 }

// open by you is loud, agent pins are the primary colour, anything already handled goes quiet.
function markerTone(p: Pin) {
  if (p.status === "fixed") return "bg-muted-foreground text-background"
  if (p.status === "taken") return "bg-secondary text-secondary-foreground"
  if (p.by === "agent") return "bg-primary text-primary-foreground"
  return "bg-destructive text-destructive-foreground"
}

const statusLabel: Record<Pin["status"], string> = { open: "Open", taken: "Agent took it", fixed: "Fixed" }
const statusVariant: Record<Pin["status"], "destructive" | "secondary" | "outline"> = {
  open: "destructive",
  taken: "secondary",
  fixed: "outline",
}

const taskTitle = (id?: string) => tasks.find((t) => t.id === id)?.title

function Marker({ p, n, selected, onSelect }: { p: Pin; n: number; selected: boolean; onSelect: () => void }) {
  const linked = taskTitle(p.taskId)
  return (
    <motion.div
      variants={pop}
      className="absolute z-10 -translate-x-1/2 -translate-y-1/2"
      style={{ left: `${p.x}%`, top: `${p.y}%` }}
    >
      <Popover>
        <PopoverTrigger
          onClick={onSelect}
          className={cn(
            "flex size-7 items-center justify-center rounded-full text-xs font-semibold shadow-md ring-2 ring-background transition-transform hover:scale-110",
            markerTone(p),
            selected && "ring-ring scale-110"
          )}
          aria-label={`Pin ${n}: ${p.note}`}
        >
          {n}
        </PopoverTrigger>
        <PopoverContent className="w-72 gap-2">
          <p className="leading-snug">{p.note}</p>
          <p className="text-muted-foreground font-mono text-xs">{p.selector}</p>
          <div className="flex flex-wrap items-center gap-1.5">
            <Badge variant="outline">{p.by === "you" ? "You" : "Agent"}</Badge>
            <Badge variant={statusVariant[p.status]}>{statusLabel[p.status]}</Badge>
          </div>
          {linked && <p className="text-muted-foreground text-xs">Task: {linked}</p>}
          <Button
            size="sm"
            variant={p.taskId ? "outline" : "default"}
            nativeButton={false}
            render={<Link to={`/w/${p.workspaceId}/tasks`} />}
          >
            {p.taskId ? "Open task" : "Make it a task"}
          </Button>
        </PopoverContent>
      </Popover>
    </motion.div>
  )
}

function PinList({
  list,
  selected,
  onSelect,
}: {
  list: Pin[]
  selected: string
  onSelect: (id: string) => void
}) {
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <motion.div variants={stagger} initial="hidden" animate="show" className="min-h-0 flex-1 overflow-y-auto">
        <ItemGroup className="gap-2">
          {list.map((p, i) => {
            const linked = taskTitle(p.taskId)
            return (
              <motion.div key={p.id} variants={rise}>
                <Item
                  variant="outline"
                  size="sm"
                  onClick={() => onSelect(p.id)}
                  className={cn("cursor-pointer items-start", selected === p.id && "ring-ring bg-muted/50 ring-2")}
                >
                  <ItemMedia>
                    <span
                      className={cn(
                        "flex size-6 items-center justify-center rounded-full text-xs font-semibold",
                        markerTone(p)
                      )}
                    >
                      {i + 1}
                    </span>
                  </ItemMedia>
                  <ItemContent className="gap-1.5">
                    <ItemTitle className="line-clamp-2 whitespace-normal">{p.note}</ItemTitle>
                    <ItemDescription className="text-muted-foreground font-mono text-xs">{p.selector}</ItemDescription>
                    <div className="flex flex-wrap items-center gap-1.5">
                      <Badge variant="outline">{p.by === "you" ? "You" : "Agent"}</Badge>
                      <Badge variant={statusVariant[p.status]}>{statusLabel[p.status]}</Badge>
                    </div>
                    {linked && <p className="text-muted-foreground truncate text-xs">Task: {linked}</p>}
                  </ItemContent>
                </Item>
              </motion.div>
            )
          })}
        </ItemGroup>
      </motion.div>
      <div className="shrink-0 space-y-2">
        <Separator />
        <div className="flex gap-2">
          <Input placeholder="Add a note to the selected pin" className="h-8" />
          <Button size="sm">Add</Button>
        </div>
        <p className="text-muted-foreground text-xs">Tap anywhere in the frame to drop a pin.</p>
      </div>
    </div>
  )
}

export function PreviewScreen() {
  const { ws } = useParams()
  const w = workspaceById(ws ?? "project-jag")
  const list = pins.filter((p) => p.workspaceId === w.id)
  // The frame opens showing the device you are holding. One read at mount, no listener.
  const [device, setDevice] = useState<Device>(() =>
    typeof window !== "undefined" && window.innerWidth < 768 ? "phone" : "desktop"
  )
  const [pinMode, setPinMode] = useState(true)
  const [selected, setSelected] = useState(list[2]?.id ?? list[0].id)
  const driving = w.agents.some((a) => a.status === "working")

  return (
    <div className="flex min-w-0 flex-col gap-4 p-4">
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <div className="flex min-w-0 flex-1 basis-full items-center gap-2 lg:basis-0">
          <Input readOnly value={w.previewUrl} className="h-8 min-w-0 flex-1 font-mono text-xs" aria-label="Preview URL" />
          <Button variant="ghost" size="icon-sm" aria-label="Reload">
            <RotateCw />
          </Button>
          <Button variant="ghost" size="icon-sm" aria-label="Open in new tab">
            <ExternalLink />
          </Button>
        </div>
        <div className="flex shrink-0 flex-wrap items-center gap-2">
          <ToggleGroup
            value={[device]}
            onValueChange={(v) => v[0] && setDevice(v[0] as Device)}
            variant="outline"
            spacing={0}
          >
            {devices.map((d) => (
              <ToggleGroupItem key={d.value} value={d.value} aria-label={d.label}>
                <d.icon data-icon="inline-start" />
                <span className="hidden sm:inline">{d.label}</span>
              </ToggleGroupItem>
            ))}
          </ToggleGroup>
          <Toggle variant="outline" pressed={pinMode} onPressedChange={setPinMode}>
            <MapPin data-icon="inline-start" />
            Pin mode
          </Toggle>
          {driving && (
            <Badge variant="secondary" className="h-8 gap-1.5 rounded-lg px-2.5">
              <span className="bg-primary size-1.5 animate-pulse rounded-full" />
              Agent is driving
            </Badge>
          )}
        </div>
      </div>

      <div className="flex min-w-0 items-start gap-4">
        <div className="flex min-w-0 flex-1 justify-center">
          <motion.div
            variants={stagger}
            initial="hidden"
            animate="show"
            className={cn(
              "bg-background relative overflow-hidden rounded-xl border shadow-sm",
              frameSize[device],
              pinMode && "cursor-crosshair"
            )}
          >
            <FakeDms narrow={device === "phone"} cols={cols[device]} />
            {pinMode &&
              list.map((p, i) => (
                <Marker key={p.id} p={p} n={i + 1} selected={selected === p.id} onSelect={() => setSelected(p.id)} />
              ))}
          </motion.div>
        </div>

        <aside className="sticky top-16 hidden max-h-[calc(100vh-6rem)] w-80 shrink-0 flex-col gap-3 lg:flex">
          <div className="flex shrink-0 items-baseline justify-between">
            <h2 className="text-sm font-medium">Pins</h2>
            <span className="text-muted-foreground text-xs">{list.length} on this page</span>
          </div>
          <PinList list={list} selected={selected} onSelect={setSelected} />
        </aside>
      </div>

      <Sheet>
        <SheetTrigger
          render={
            <Button className="fixed right-4 bottom-20 z-40 shadow-lg md:bottom-6 lg:hidden">
              <MapPin data-icon="inline-start" />
              Pins ({list.length})
            </Button>
          }
        />
        <SheetContent side="bottom" className="flex h-[80vh] flex-col gap-3 p-4">
          <SheetHeader className="shrink-0 p-0">
            <SheetTitle>Pins on this page</SheetTitle>
          </SheetHeader>
          <PinList list={list} selected={selected} onSelect={setSelected} />
        </SheetContent>
      </Sheet>
    </div>
  )
}
