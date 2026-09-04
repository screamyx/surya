// The one toolbar, shared by both engines. Decision 21: "The pane shows which engine is
// active." So the switch sits first, before the address, and everything to its right -
// device, pin mode, the driving chip - stays put when the engine changes.
import { motion } from "motion/react"
import { AppWindow, Check, Globe, MapPin, Monitor, MoreVertical, RotateCw, Smartphone, Tablet } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Toggle } from "@/components/ui/toggle"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { breathe } from "@/motion"
import { cn } from "@/lib/utils"
import {
  connections,
  sites,
  type Connection,
  type Engine,
  type SiteId,
} from "@/screens/session/preview/browser-data"

export type Device = "phone" | "tablet" | "desktop"

const devices: { value: Device; label: string; icon: typeof Monitor }[] = [
  { value: "phone", label: "Phone", icon: Smartphone },
  { value: "tablet", label: "Tablet", icon: Tablet },
  { value: "desktop", label: "Desktop", icon: Monitor },
]

const engines: { value: Engine; label: string; icon: typeof Globe }[] = [
  { value: "app", label: "App", icon: AppWindow },
  { value: "browser", label: "Browser", icon: Globe },
]

function EngineSwitch({ engine, onEngine }: { engine: Engine; onEngine: (e: Engine) => void }) {
  return (
    <ToggleGroup
      value={[engine]}
      onValueChange={(v) => v[0] && onEngine(v[0] as Engine)}
      variant="outline"
      size="sm"
      spacing={0}
      aria-label="Preview engine"
      className="shrink-0"
    >
      {engines.map((e) => (
        <ToggleGroupItem key={e.value} value={e.value} aria-label={`${e.label} engine`}>
          <e.icon data-icon="inline-start" />
          {e.label}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  )
}

function AddressMenu({ engine, site, connection, onSite, onConnection }: {
  engine: Engine
  site: SiteId
  connection: Connection
  onSite: (s: SiteId) => void
  onConnection: (c: Connection) => void
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={<Button variant="ghost" size="icon-sm" aria-label="Address menu" />}
      >
        <MoreVertical />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-72">
        {engine === "browser" && (
          <>
            <DropdownMenuGroup>
            <DropdownMenuLabel>Recent pages</DropdownMenuLabel>
            {sites.map((s) => (
              <DropdownMenuItem key={s.id} onClick={() => onSite(s.id)} className="items-start gap-2">
                <Check className={cn("mt-0.5 size-4 shrink-0", site !== s.id && "opacity-0")} />
                <span className="flex min-w-0 flex-col">
                  <span className="truncate">{s.title}</span>
                  <span className="text-muted-foreground truncate font-mono text-xs">{s.host}</span>
                  <span className="text-muted-foreground text-xs">Visited {s.visited}</span>
                </span>
              </DropdownMenuItem>
            ))}
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
            <DropdownMenuLabel>Where this Chrome runs</DropdownMenuLabel>
            {connections.map((c) => (
              <DropdownMenuItem
                key={c.value}
                onClick={() => onConnection(c.value)}
                className="items-start gap-2"
              >
                <Check className={cn("mt-0.5 size-4 shrink-0", connection !== c.value && "opacity-0")} />
                <span className="flex min-w-0 flex-col">
                  <span className="truncate">{c.label}</span>
                  <span className="text-muted-foreground text-xs leading-snug whitespace-normal">
                    {c.detail}
                  </span>
                </span>
              </DropdownMenuItem>
            ))}
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
          </>
        )}
        <DropdownMenuItem>Open in a new tab</DropdownMenuItem>
        <DropdownMenuItem>Copy the address</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

export function PreviewToolbar(props: {
  engine: Engine
  onEngine: (e: Engine) => void
  url: string
  site: SiteId
  onSite: (s: SiteId) => void
  connection: Connection
  onConnection: (c: Connection) => void
  account: string
  signedIn: boolean
  device: Device
  onDevice: (d: Device) => void
  pinMode: boolean
  onPinMode: (v: boolean) => void
  driving: boolean
}) {
  const { engine, url, account, signedIn, driving } = props
  return (
    <div className="flex min-w-0 shrink-0 flex-wrap items-center gap-2 border-b px-3 py-2">
      <div className="flex min-w-0 flex-1 basis-full flex-wrap items-center gap-1.5">
        <EngineSwitch engine={engine} onEngine={props.onEngine} />
        <Input
          readOnly
          value={url}
          className="h-8 min-w-0 flex-1 basis-40 font-mono text-xs"
          aria-label={engine === "app" ? "App address" : "Browser address"}
        />
        <Button variant="ghost" size="icon-sm" aria-label="Reload" className="ml-auto"><RotateCw /></Button>
        <AddressMenu
          engine={engine}
          site={props.site}
          connection={props.connection}
          onSite={props.onSite}
          onConnection={props.onConnection}
        />
      </div>

      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <ToggleGroup
          value={[props.device]}
          onValueChange={(v) => v[0] && props.onDevice(v[0] as Device)}
          variant="outline"
          size="sm"
          spacing={0}
          aria-label="Device"
        >
          {devices.map((d) => (
            <ToggleGroupItem key={d.value} value={d.value} aria-label={d.label}>
              <d.icon />
            </ToggleGroupItem>
          ))}
        </ToggleGroup>
        <Toggle variant="outline" size="sm" pressed={props.pinMode} onPressedChange={props.onPinMode}>
          <MapPin data-icon="inline-start" />Pin mode
        </Toggle>
        {engine === "browser" && (
          <Badge variant="outline" className="min-w-0 gap-1.5">
            {signedIn ? (
              <>
                <span className="shrink-0">Signed in as</span>
                <span className="min-w-0 truncate font-mono">{account}</span>
              </>
            ) : (
              <span className="min-w-0 truncate">{account}</span>
            )}
          </Badge>
        )}
        {driving && (
          <Badge variant="secondary" className="gap-1.5">
            <motion.span
              variants={breathe}
              initial="rest"
              animate="live"
              className="bg-primary size-1.5 rounded-full"
            />
            Agent is driving
          </Badge>
        )}
      </div>
    </div>
  )
}
