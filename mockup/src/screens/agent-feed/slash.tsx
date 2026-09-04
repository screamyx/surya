// The slash palette. Opens when the composer starts with "/". The list is what Claude Code reported for this
// workspace: built-ins, user skills, project skills, plugins. surya adds nothing of its own.
import type { ComponentType } from "react"
import { motion } from "motion/react"
import { Badge } from "@/components/ui/badge"
import { Kbd } from "@/components/ui/kbd"
import { pop } from "@/motion"
import { slashCommands, type SlashCommand, type SlashSource } from "@/data"

type Cmp = ComponentType<any>
const order: SlashSource[] = ["project skill", "user skill", "plugin", "built-in"]

export function SlashPalette({ query, onPick, Command, CommandList, CommandGroup, CommandItem, CommandEmpty }: {
  query: string; onPick: (name: string) => void; Command: Cmp; CommandList: Cmp; CommandGroup: Cmp; CommandItem: Cmp; CommandEmpty: Cmp
}) {
  const q = query.toLowerCase()
  const hits = slashCommands.filter((c) => c.name.includes(q) || c.description.toLowerCase().includes(q))
  const groups = order.map((src) => ({ src, items: hits.filter((c) => c.source === src) })).filter((g) => g.items.length)
  return (
    <motion.div variants={pop} initial="hidden" animate="show" className="absolute inset-x-0 bottom-full z-30 mb-2">
      <Command shouldFilter={false} className="bg-popover text-popover-foreground rounded-xl border shadow-lg">
        <div className="text-muted-foreground flex items-center justify-between border-b px-3 py-2 text-xs">
          <span>{hits.length} of {slashCommands.length} commands{q && <> matching <span className="text-foreground font-mono">/{q}</span></>}</span>
          <span className="hidden items-center gap-1 sm:flex"><Kbd>↑</Kbd><Kbd>↓</Kbd> pick <Kbd>↵</Kbd> insert <Kbd>esc</Kbd> close</span>
        </div>
        <CommandList className="max-h-72">
          <CommandEmpty>No command matches. Send it as text instead.</CommandEmpty>
          {groups.map((g) => (
            <CommandGroup key={g.src} heading={label(g.src)}>
              {g.items.map((c) => <Row key={c.name} c={c} onPick={onPick} CommandItem={CommandItem} />)}
            </CommandGroup>
          ))}
        </CommandList>
      </Command>
    </motion.div>
  )
}

function label(s: SlashSource) {
  return { "project skill": "This workspace", "user skill": "Your skills", plugin: "Plugins", "built-in": "Claude Code" }[s]
}

function Row({ c, onPick, CommandItem }: { c: SlashCommand; onPick: (n: string) => void; CommandItem: Cmp }) {
  return (
    <CommandItem value={c.name} onSelect={() => onPick(c.name)} className="gap-3">
      <span className="shrink-0 font-mono text-[13px] whitespace-nowrap">/{c.name}</span>
      {c.args && <span className="text-muted-foreground hidden truncate font-mono text-xs sm:inline">{c.args}</span>}
      <span className="text-muted-foreground ml-auto hidden max-w-[50%] truncate text-xs sm:inline">{c.description}</span>
      <Badge variant="outline" className="ml-auto h-4 shrink-0 px-1.5 text-[10px] sm:hidden">{c.source}</Badge>
    </CommandItem>
  )
}
