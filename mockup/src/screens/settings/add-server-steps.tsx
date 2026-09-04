// The four steps of the add-server flow. Decision 18: plain SSH from any daemon,
// a copy-paste installer line is always the fallback, nothing assumes one person's network.
import { useState } from "react"
import { AnimatePresence, motion } from "motion/react"
import { Check, Copy, KeyRound, Search, X } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group"
import { Item, ItemActions, ItemContent, ItemDescription, ItemGroup, ItemTitle } from "@/components/ui/item"
import { Progress } from "@/components/ui/progress"
import { Spinner } from "@/components/ui/spinner"
import { osIcon } from "@/screens/settings/servers"
import { servers } from "@/data"
import { pop, rise, stagger } from "@/motion"
import { cn } from "@/lib/utils"

type Os = keyof typeof osIcon

const machines: { name: string; ip: string; os: Os; installed: boolean }[] = [
  { name: "laptop-2", ip: "192.168.1.31", os: "mac", installed: false },
  { name: "mini-01", ip: "192.168.1.24", os: "linux", installed: false },
  { name: "studio", ip: "192.168.1.10", os: "linux", installed: true },
]

export const installLine = "curl -fsSL https://surya.dev/install | sh"

export function StepPickMachine({ host, setHost }: { host: string; setHost: (v: string) => void }) {
  return (
    <div className="flex flex-col gap-4">
      <InputGroup>
        <InputGroupAddon><Search /></InputGroupAddon>
        <InputGroupInput value={host} onChange={(e) => setHost(e.target.value)} placeholder="hostname or IP" aria-label="hostname or IP" className="font-mono" />
      </InputGroup>

      <div className="flex flex-col gap-2">
        <span className="text-muted-foreground text-xs">Machines seen on your network</span>
        <motion.div variants={stagger} initial="hidden" animate="show">
          <ItemGroup className="gap-1">
            {machines.map((m) => {
              const Icon = osIcon[m.os]
              const picked = host === m.name
              return (
                <motion.div key={m.name} variants={rise}>
                  <Item
                    variant="outline"
                    role="button"
                    tabIndex={0}
                    onClick={() => setHost(m.name)}
                    onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setHost(m.name) } }}
                    className={cn("hover:bg-muted cursor-pointer flex-nowrap transition-colors", picked && "border-primary bg-muted")}
                  >
                    <Icon className="text-muted-foreground size-4 shrink-0" />
                    <ItemContent className="min-w-0">
                      <ItemTitle className="truncate">{m.name}</ItemTitle>
                      <ItemDescription className="font-mono">{m.ip}</ItemDescription>
                    </ItemContent>
                    <ItemActions>
                      <Badge variant={m.installed ? "secondary" : "outline"} className="font-normal">
                        surya: {m.installed ? "installed" : "not installed"}
                      </Badge>
                    </ItemActions>
                  </Item>
                </motion.div>
              )
            })}
          </ItemGroup>
        </motion.div>
      </div>

      <p className="text-muted-foreground text-sm">Any machine you can reach over SSH. LAN, VPN, or a tunnel, surya does not care which.</p>
    </div>
  )
}

type CheckState = "ok" | "failed" | "running"

function CheckIcon({ state }: { state: CheckState }) {
  if (state === "running") return <Spinner className="text-muted-foreground" />
  if (state === "failed") return <X className="text-destructive size-4" />
  return <Check className="text-primary size-4" />
}

export function StepChecks({ host, loggedIn, onLogin }: { host: string; loggedIn: boolean; onLogin: () => void }) {
  const [fixing, setFixing] = useState(false)
  const rows: { id: string; label: string; note: string; state: CheckState }[] = [
    { id: "ssh", label: "Reachable over SSH", note: `key accepted as you@${host}`, state: "ok" },
    { id: "claude", label: "Claude Code installed", note: "claude 2.1.260 on PATH", state: "ok" },
    {
      id: "login",
      label: "Claude Code logged in",
      note: loggedIn ? "claude.ai OAuth, Max plan" : "no token on this machine",
      state: loggedIn ? "ok" : "failed",
    },
    { id: "disk", label: "Disk space", note: "48 GB free, surya needs 200 MB", state: "ok" },
  ]

  return (
    <motion.div variants={stagger} initial="hidden" animate="show" className="flex flex-col gap-2">
      {rows.map((r) => (
        <motion.div key={r.id} variants={rise} className="flex flex-col gap-2">
          <Item variant="outline" className={cn("flex-nowrap gap-3", r.state === "failed" && "border-destructive/40")}>
            <CheckIcon state={r.state} />
            <ItemContent className="min-w-0">
              <ItemTitle className="truncate">{r.label}</ItemTitle>
              <ItemDescription className="truncate font-mono">{r.note}</ItemDescription>
            </ItemContent>
            {r.state === "failed" && (
              <ItemActions>
                <Button variant="outline" size="sm" className="h-9 md:h-7" onClick={() => setFixing(true)}>Fix</Button>
              </ItemActions>
            )}
          </Item>

          <AnimatePresence>
            {r.id === "login" && r.state === "failed" && fixing && (
              <motion.div variants={pop} initial="hidden" animate="show" exit="hidden">
                <Card size="sm" className="bg-muted/40">
                  <CardContent className="flex flex-col gap-3">
                    <p className="text-sm leading-relaxed">
                      surya starts <code className="font-mono">claude setup-token</code> on the machine and shows you the login link. Tap it, sign in, come back.
                    </p>
                    <Button className="h-9 w-fit md:h-8" onClick={onLogin}>
                      <KeyRound data-icon="inline-start" />Start login
                    </Button>
                  </CardContent>
                </Card>
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>
      ))}
    </motion.div>
  )
}

const log = (host: string) => [
  `$ ssh ${host} 'curl -fsSL https://surya.dev/install | sh'`,
  "downloading surya 0.1.0, 14.2 MB",
  "checksum ok",
  "installing to /usr/local/bin/surya",
  "writing ~/.config/surya/daemon.toml",
  "found claude 2.1.260 on PATH",
  "minting a daemon token with claude setup-token",
  "adding a boot service so it survives a restart",
  "starting the daemon on port 4791",
  `registering with ${servers.find((s) => s.home)!.name}, your home server`,
]

export function StepInstall({ host }: { host: string }) {
  const lines = log(host)
  return (
    <div className="flex flex-col gap-4">
      <Progress value={78}>
        <span className="text-sm font-medium">Installing on {host}</span>
        <span className="text-muted-foreground ml-auto text-sm tabular-nums">78%</span>
      </Progress>

      <motion.div variants={stagger} initial="hidden" animate="show" className="bg-muted rounded-lg p-3">
        {lines.map((l, i) => (
          <motion.p
            key={l}
            variants={rise}
            className={cn("flex items-start gap-2 font-mono text-xs leading-relaxed", i === lines.length - 1 ? "text-foreground" : "text-muted-foreground")}
          >
            {i === lines.length - 1 && <Spinner className="mt-0.5 size-3 shrink-0" />}
            <span className="min-w-0 break-words">{l}</span>
          </motion.p>
        ))}
      </motion.div>

      <div className="flex flex-col gap-2">
        <p className="text-muted-foreground text-sm">SSH not possible? Run this on the machine instead:</p>
        <div className="bg-muted flex items-center gap-2 rounded-lg p-2">
          <code className="min-w-0 flex-1 truncate font-mono text-xs">{installLine}</code>
          <Button variant="ghost" size="sm" className="h-8 shrink-0"><Copy data-icon="inline-start" />Copy</Button>
        </div>
        <p className="text-muted-foreground text-sm">Waiting for it to answer…</p>
      </div>
    </div>
  )
}

export function StepDone({ host, onClose }: { host: string; onClose: () => void }) {
  return (
    <motion.div variants={pop} initial="hidden" animate="show" className="flex flex-col items-center gap-4 py-8 text-center">
      <span className="bg-primary text-primary-foreground grid size-16 place-items-center rounded-full">
        <Check className="size-8" />
      </span>
      <div className="flex flex-col gap-1">
        <p className="font-heading text-lg font-medium">{host} is online.</p>
        <p className="text-muted-foreground text-sm">0 workspaces.</p>
      </div>
      <div className="flex w-full flex-col gap-2 sm:w-auto sm:flex-row">
        <Button className="h-11 sm:h-9">Add a workspace</Button>
        <Button variant="outline" className="h-11 sm:h-9" onClick={onClose}>Close</Button>
      </div>
    </motion.div>
  )
}
