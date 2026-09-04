// The filter strip scrolls sideways on a phone, and a hidden overflow needs a cue a
// sighted person can see. So the strip measures itself: when there is more to the right
// it fades that edge and puts an arrow on it that scrolls. An aria-label alone would
// serve screen readers only.
import { useCallback, useEffect, useRef, useState } from "react"
import { ChevronLeft, ChevronRight } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"

export type Edge = { start: boolean; end: boolean }

export function FilterStrip<T extends string>({ value, onValueChange, tabs }: {
  value: T
  onValueChange: (v: T) => void
  tabs: { id: T; label: string; count: number; hot: boolean }[]
}) {
  const ref = useRef<HTMLDivElement>(null)
  const [edge, setEdge] = useState<Edge>({ start: false, end: false })

  const measure = useCallback(() => {
    const el = ref.current
    if (!el) return
    setEdge({
      start: el.scrollLeft > 1,
      end: el.scrollLeft + el.clientWidth < el.scrollWidth - 1,
    })
  }, [])

  useEffect(() => {
    const el = ref.current
    if (!el) return
    measure()
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  }, [measure])

  const nudge = (dir: 1 | -1) => {
    const el = ref.current
    if (!el) return
    el.scrollBy({ left: dir * el.clientWidth * 0.7, behavior: "smooth" })
  }

  return (
    <Tabs value={value} onValueChange={(v) => onValueChange(v as T)}>
      <div className="relative">
        <div ref={ref} onScroll={measure} className="overflow-x-auto">
          <TabsList className="w-max justify-start">
            {tabs.map((t) => (
              <TabsTrigger key={t.id} value={t.id}>
                {t.label}
                <Badge variant={t.hot ? "destructive" : "secondary"} className="tnum px-1.5 text-xs">{t.count}</Badge>
              </TabsTrigger>
            ))}
          </TabsList>
        </div>

        {edge.start && (
          <div className="from-muted pointer-events-none absolute inset-y-0 left-0 flex items-center rounded-l-lg bg-gradient-to-r from-50% to-transparent pr-6 pl-0.5">
            <Button size="icon-xs" variant="ghost" className="pointer-events-auto" onClick={() => nudge(-1)} aria-label="Show earlier filters">
              <ChevronLeft />
            </Button>
          </div>
        )}
        {edge.end && (
          <div className="from-muted pointer-events-none absolute inset-y-0 right-0 flex items-center rounded-r-lg bg-gradient-to-l from-50% to-transparent pr-0.5 pl-6">
            <Button size="icon-xs" variant="ghost" className="pointer-events-auto" onClick={() => nudge(1)} aria-label="Show more filters">
              <ChevronRight />
            </Button>
          </div>
        )}
      </div>
    </Tabs>
  )
}
