// The live view, with pins. A pane of the session, not a route of its own.
//
// Decision 21 gives it two engines. App is proxy plus iframe of the workspace dev server:
// DOM pins, native scroll, element picking. Browser is a real Chrome the daemon owns,
// painted here as a frame stream, because "a browsable third party site is non-negotiable"
// and most sites forbid framing. The pane says which one is live; the pins look the same in
// both, because in both the daemon resolves a selector for the element under the click.
//
// Picking a pin writes it into the composer as structured context for the agent. Windsurf
// calls the same move "Send element", which "turns a visual selection or console error into
// structured prompt context beside the agent" (docs/research/windsurf.md, job 6).
import { useLayoutEffect, useRef, useState } from "react"
import { AnimatePresence, motion } from "motion/react"
import { MapPin } from "lucide-react"
import { ScrollArea } from "@/components/ui/scroll-area"
import { pins, type Pin, type Workspace } from "@/data"
import { beat, stagger, swap } from "@/motion"
import { cn } from "@/lib/utils"
import { FakeDms } from "@/screens/session/fake-dms"
import { useCompose } from "@/screens/session/compose"
import { PreviewToolbar, type Device } from "@/screens/session/preview/toolbar"
import { Marker, PinList } from "@/screens/session/preview/pins"
import { asPrompt, selectorAt } from "@/screens/session/preview/pin-utils"
import { BrowserFrame } from "@/screens/session/preview/browser-frame"
import {
  browserPins,
  paints,
  siteById,
  type Connection,
  type Engine,
  type SiteId,
} from "@/screens/session/preview/browser-data"

const frameSize: Record<Device, string> = {
  phone: "w-[390px] max-w-full aspect-[390/720]",
  tablet: "w-[820px] max-w-full aspect-[4/3]",
  desktop: "w-full aspect-[16/10]",
}

const cols: Record<Device, 2 | 3 | 4> = { phone: 2, tablet: 3, desktop: 4 }

// Pins belong to a page, not to the pane. Switching engine or address swaps the whole set,
// the way it would in the real thing.
const surfaceKey = (engine: Engine, site: SiteId) => (engine === "app" ? "app" : `browser:${site}`)

export function PreviewPane({ ws, compact }: { ws: Workspace; compact: boolean }) {
  const { send } = useCompose()
  const [engine, setEngine] = useState<Engine>("app")
  // The Browser engine remembers its last address, so coming back from App lands where you left.
  const [site, setSite] = useState<SiteId>("ads")
  const [connection, setConnection] = useState<Connection>("daemon")
  const [device, setDevice] = useState<Device>(compact ? "phone" : "desktop")
  const [pinMode, setPinMode] = useState(true)
  const [dropped, setDropped] = useState<Record<string, Pin[]>>({})
  const [fresh, setFresh] = useState("")
  const [selected, setSelected] = useState("")

  const current = siteById(site)
  const key = surfaceKey(engine, site)
  const base = engine === "app" ? pins.filter((p) => p.workspaceId === ws.id) : browserPins[site]
  const list = [...base, ...(dropped[key] ?? [])]
  const url = engine === "app" ? ws.previewUrl : current.url
  const driving = ws.agents.some((a) => a.status === "working")
  const open = list.filter((p) => p.status === "open").length
  const live = engine === "app" || paints(connection, compact)

  const sendPin = (p: Pin, n: number) => {
    setSelected(p.id)
    send(asPrompt(p, n, url))
  }

  // The daemon asks the page which element sits under the click, then stores selector plus
  // position. Here the DOM answers directly, which is the same answer Chrome gives.
  const drop = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!pinMode || !live) return
    const target = e.target as Element
    // A popover renders through a portal, and React still bubbles its clicks up this tree.
    // Only a click that really landed inside the frame drops a pin.
    if (!e.currentTarget.contains(target)) return
    if (target.closest("[data-pin-marker]")) return
    const box = e.currentTarget.getBoundingClientRect()
    const pin: Pin = {
      id: `drop-${key}-${Date.now()}`,
      workspaceId: ws.id,
      x: Math.round(((e.clientX - box.left) / box.width) * 1000) / 10,
      y: Math.round(((e.clientY - box.top) / box.height) * 1000) / 10,
      selector: selectorAt(target),
      note: "",
      by: "you",
      status: "open",
    }
    setDropped((d) => ({ ...d, [key]: [...(d[key] ?? []), pin] }))
    setFresh(pin.id)
    setSelected(pin.id)
  }

  // A pin is a selector plus a position, and the selector is the truth. Measuring the
  // element it names keeps the bubble on its target when the frame changes width, device or
  // page, instead of parking it wherever the stored percentage happens to land.
  const frame = useRef<HTMLDivElement>(null)
  const [anchors, setAnchors] = useState<Record<string, { x: number; y: number }>>({})

  useLayoutEffect(() => {
    const el = frame.current
    if (!el) return
    const measure = () => {
      const box = el.getBoundingClientRect()
      if (!box.width || !box.height) return
      const hooks = [...el.querySelectorAll("[data-el]")]
      const next: Record<string, { x: number; y: number }> = {}
      for (const p of list) {
        const hit = hooks.find((h) => h.getAttribute("data-el") === p.selector)
        if (!hit) continue
        const r = hit.getBoundingClientRect()
        next[p.id] = {
          x: ((r.left + r.width / 2 - box.left) / box.width) * 100,
          y: ((r.top + r.height / 2 - box.top) / box.height) * 100,
        }
      }
      setAnchors(next)
    }
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(el)
    return () => observer.disconnect()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, device, connection, live, list.length])

  const noteFor = (id: string, text: string) =>
    setDropped((d) => ({
      ...d,
      [key]: (d[key] ?? []).map((p) => (p.id === id ? { ...p, note: text } : p)),
    }))

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      <PreviewToolbar
        engine={engine}
        onEngine={setEngine}
        url={url}
        site={site}
        onSite={setSite}
        connection={connection}
        onConnection={setConnection}
        account={current.account}
        signedIn={current.signIn === "account"}
        device={device}
        onDevice={setDevice}
        pinMode={pinMode}
        onPinMode={setPinMode}
        driving={driving}
      />

      <ScrollArea className="min-h-0 flex-1">
        <div className="flex min-w-0 justify-center p-4">
          <motion.div
            variants={stagger}
            initial="hidden"
            animate="show"
            ref={frame}
            onClick={drop}
            className={cn(
              "bg-background relative overflow-hidden rounded-xl shadow-lift",
              // A frame that cannot paint keeps no device shape: sizing a message to a phone
              // aspect leaves a screen of nothing above it.
              live ? frameSize[device] : "w-full",
              pinMode && live && "cursor-crosshair",
            )}
          >
            {/* The old frame leaves before the new one arrives, so nothing crosses over. */}
            <AnimatePresence mode="wait" initial={false}>
              <motion.div
                key={key + connection}
                variants={swap}
                initial="hidden"
                animate="show"
                exit="exit"
                className="size-full"
              >
                {engine === "app" ? (
                  <FakeDms narrow={device === "phone"} cols={cols[device]} />
                ) : (
                  <BrowserFrame
                    site={current}
                    connection={connection}
                    narrow={device === "phone"}
                    compact={compact}
                    onUseDaemon={() => setConnection("daemon")}
                  />
                )}
              </motion.div>
            </AnimatePresence>

            {pinMode && live && list.map((p, i) => (
              <Marker
                key={p.id}
                p={anchors[p.id] ? { ...p, ...anchors[p.id] } : p}
                n={i + 1}
                selected={selected === p.id}
                fresh={fresh === p.id}
                onSelect={() => setSelected(p.id)}
                onSend={() => sendPin(p, i + 1)}
                onNote={(text) => noteFor(p.id, text)}
              />
            ))}
          </motion.div>
        </div>

        <div className="space-y-3 px-4 pb-4">
          <div className="flex items-baseline justify-between gap-2">
            <h2 className="text-lg font-medium">Pins</h2>
            {list.length > 0 && (
              <motion.span
                key={list.length}
                variants={beat}
                initial="rest"
                animate="hit"
                className="text-muted-foreground tnum text-xs"
              >
                {open} open of {list.length}
              </motion.span>
            )}
          </div>
          {list.length > 0 ? (
            <PinList list={list} selected={selected} onSelect={setSelected} onSend={sendPin} />
          ) : (
            <div className="bg-card flex flex-col items-center gap-2 rounded-xl border px-6 py-10 text-center">
              <MapPin className="text-muted-foreground size-5" />
              <p className="font-medium">No pins on this page yet</p>
              <p className="text-muted-foreground max-w-sm text-sm">
                Point at anything in the frame and the note goes to {ws.agents[0]?.name ?? "the agent"} with
                the element it belongs to. Pin mode has to be on.
              </p>
            </div>
          )}
        </div>
      </ScrollArea>

      <footer className="bg-background text-muted-foreground flex shrink-0 items-center gap-2 border-t px-4 py-2.5 text-xs">
        <MapPin className="size-3.5 shrink-0" />
        <span className="min-w-0">
          {!pinMode
            ? "Pin mode is off. Turn it on to point at an element."
            : !live
              ? "Nothing to point at until this Chrome paints."
              : engine === "app"
                ? "Tap the frame to drop a pin. Send one and it lands in the composer."
                : "Tap the page to drop a pin. The daemon names the element and sends the selector."}
        </span>
      </footer>
    </div>
  )
}
