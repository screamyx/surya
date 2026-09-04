// The right column: start something, then the machines. The inventory that used to
// own Home lives here now, demoted to a list, because Home answers "what now".
import { Link } from "react-router"
import { ArrowRight, Server as ServerIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { flatten, servers, workspaces } from "@/data"
import { cn } from "@/lib/utils"

export function StartBox() {
  return (
    <section className="bg-card shadow-raise rounded-xl p-5">
      <h2 className="text-base font-medium">Start something</h2>
      <p className="text-muted-foreground mt-0.5 text-sm">One sentence is enough. An agent picks it up.</p>
      <Textarea placeholder="Add a Ship button to the result page" className="mt-3 min-h-20" />
      <Button variant="outline" nativeButton={false} className="mt-3 h-11 w-full sm:h-9" render={<Link to="/new" />}>
        Start
        <ArrowRight data-icon="inline-end" />
      </Button>
    </section>
  )
}

export function MachineList() {
  return (
    <section className="rounded-xl border p-5">
      <h2 className="u-overline text-muted-foreground">Machines</h2>
      <ul className="mt-3 flex flex-col gap-3">
        {servers.map((s) => {
          const mine = workspaces.filter((w) => w.serverId === s.id)
          const off = s.state !== "online"
          return (
            <li key={s.id} className="flex flex-col gap-1.5">
              <div className="flex min-w-0 items-center gap-2">
                <ServerIcon className={cn("size-3.5 shrink-0", off ? "text-muted-foreground" : "text-foreground")} />
                <span className={cn("truncate text-sm font-medium", off && "text-muted-foreground")}>{s.name}</span>
                {s.home && <span className="text-muted-foreground text-2xs">home</span>}
                <span className="ml-auto flex shrink-0 items-center gap-1.5">
                  <span className={cn("size-1.5 rounded-full", off ? "bg-border-strong" : "bg-ok")} />
                  <span className="text-muted-foreground text-xs">{s.state}</span>
                </span>
              </div>
              {mine.length === 0 ? (
                <p className="text-muted-foreground pl-5.5 text-xs">Not reachable. Check it in Settings.</p>
              ) : (
                <ul className="flex flex-col gap-0.5 pl-5.5">
                  {mine.map((w) => (
                    <li key={w.id}>
                      <Link
                        to={`/w/${w.id}/agents`}
                        className="hover:text-foreground text-muted-foreground flex min-w-0 items-baseline gap-2 text-xs"
                      >
                        <span className="truncate font-mono">{w.name}</span>
                        <span className="text-muted-foreground tnum ml-auto shrink-0">
                          {flatten(w.agents).length}
                        </span>
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
            </li>
          )
        })}
      </ul>
      <Button variant="ghost" size="sm" nativeButton={false} className="text-muted-foreground mt-3 -ml-2" render={<Link to="/settings" />}>
        Manage servers
        <ArrowRight data-icon="inline-end" />
      </Button>
    </section>
  )
}
