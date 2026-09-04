// What the Browser engine paints. Decision 21: "a real Chrome owned by the daemon, painted
// into the pane as a frame stream over the Chrome DevTools Protocol, with mouse, keyboard
// and scroll forwarded back."
//
// So the frame carries a tab strip: this is a browser, not an embed. And it carries the one
// state the decision insists on being honest about - desktop Chrome cannot paint on a phone.
import { Monitor } from "lucide-react"
import { Button } from "@/components/ui/button"
import { SiteAds } from "@/screens/session/preview/site-ads"
import { SiteDocs } from "@/screens/session/preview/site-docs"
import { paints, profile, type BrowserSite, type Connection } from "@/screens/session/preview/browser-data"

function TabStrip({ site, connection }: { site: BrowserSite; connection: Connection }) {
  return (
    <div className="bg-canvas flex min-w-0 shrink-0 items-center gap-1.5 border-b px-2 py-1">
      <span className="bg-card flex min-w-0 items-center gap-1.5 rounded-md border px-2 py-0.5">
        <span className="bg-muted-foreground size-2 shrink-0 rounded-sm" />
        <span className="truncate text-2xs">{site.title}</span>
      </span>
      <span className="text-muted-foreground ml-auto shrink-0 truncate text-2xs">
        {connection === "daemon" ? profile.name : "Your desktop"}
      </span>
    </div>
  )
}

// The honest dead end. Not an error the user caused: a limit of the connection they picked.
function CannotPaint({ onUseDaemon }: { onUseDaemon: () => void }) {
  return (
    <div className="bg-canvas flex w-full flex-col items-center justify-center gap-3 px-6 py-12 text-center">
      <Monitor className="text-muted-foreground size-6" />
      <div className="space-y-1">
        <p className="font-medium">Your desktop Chrome cannot paint here</p>
        <p className="text-muted-foreground mx-auto max-w-xs text-sm leading-snug">
          That tab lives on your desktop and only draws on that screen. The daemon's Chrome runs on the
          server and paints on any device, this one included.
        </p>
      </div>
      <Button size="sm" onClick={onUseDaemon}>Switch to the daemon's Chrome</Button>
    </div>
  )
}

export function BrowserFrame({ site, connection, narrow, compact, onUseDaemon }: {
  site: BrowserSite
  connection: Connection
  narrow: boolean
  compact: boolean
  onUseDaemon: () => void
}) {
  return (
    <div className="bg-background flex size-full min-w-0 flex-col overflow-hidden">
      <TabStrip site={site} connection={connection} />
      <div className="min-h-0 min-w-0 flex-1">
        {!paints(connection, compact) ? (
          <CannotPaint onUseDaemon={onUseDaemon} />
        ) : site.id === "ads" ? (
          <SiteAds narrow={narrow} />
        ) : (
          <SiteDocs narrow={narrow} />
        )}
      </div>
    </div>
  )
}
