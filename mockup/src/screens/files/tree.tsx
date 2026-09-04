// The workspace file tree. Folders collapse, files open in the editor, and anything an
// agent touched sits lit up on bg-accent so you can see the work landing.
import { motion } from "motion/react"
import { ChevronRight, File, Folder, Search } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { cn } from "@/lib/utils"
import type { FileNode } from "@/data"
import { glow, rise, stagger } from "@/motion"

// Padding that grows with the depth, from the spacing token so a restyle stays a token change.
const indent = (depth: number, isFile: boolean) => ({
  // Files sit where a folder's name sits, so the column of names stays straight.
  paddingInlineStart: `calc(var(--spacing) * ${depth * 3 + 2 + (isFile ? 6 : 0)})`,
})

function filterTree(nodes: FileNode[], query: string): FileNode[] {
  const q = query.trim().toLowerCase()
  if (!q) return nodes
  const walk = (list: FileNode[]): FileNode[] =>
    list.flatMap((n) => {
      if (n.kind === "file") return n.name.toLowerCase().includes(q) ? [n] : []
      const kids = walk(n.children ?? [])
      if (kids.length) return [{ ...n, children: kids }]
      return n.name.toLowerCase().includes(q) ? [n] : []
    })
  return walk(nodes)
}

function TouchedMark({ touched }: { touched: FileNode["touched"] }) {
  if (!touched) return null
  return (
    <span className="ml-auto flex shrink-0 items-center gap-1.5">
      {touched === "agent" && <span className="bg-primary size-1.5 animate-pulse rounded-full" />}
      <Badge variant="outline" className="hidden group-hover/row:inline-flex">{touched}</Badge>
    </span>
  )
}

function FileRow({ node, depth, selected, onSelect }: { node: FileNode; depth: number; selected: string; onSelect: (path: string) => void }) {
  const active = selected === node.path
  return (
    <motion.div variants={rise}>
      <motion.button
        type="button"
        onClick={() => onSelect(node.path)}
        variants={node.touched && !active ? glow : undefined}
        initial={node.touched && !active ? "idle" : undefined}
        animate={node.touched && !active ? "touched" : undefined}
        style={indent(depth, true)}
        className={cn(
          "group/row flex w-full items-center gap-2 rounded-md py-2 pr-2 text-left text-sm transition-colors md:py-1.5",
          active ? "bg-primary text-primary-foreground font-medium" : "hover:bg-muted",
        )}
      >
        <File className="text-muted-foreground size-4 shrink-0" />
        <span className="truncate">{node.name}</span>
        <TouchedMark touched={node.touched} />
      </motion.button>
    </motion.div>
  )
}

function Node({ node, depth, selected, onSelect }: { node: FileNode; depth: number; selected: string; onSelect: (path: string) => void }) {
  if (node.kind === "file") return <FileRow node={node} depth={depth} selected={selected} onSelect={onSelect} />
  return (
    <Collapsible defaultOpen>
      <motion.div variants={rise}>
        <CollapsibleTrigger
          style={indent(depth, false)}
          className="group/dir hover:bg-muted flex w-full items-center gap-2 rounded-md py-2 pr-2 text-left text-sm transition-colors md:py-1.5"
        >
          <ChevronRight className="text-muted-foreground size-4 shrink-0 transition-transform group-data-open/dir:rotate-90" />
          <Folder className="text-muted-foreground size-4 shrink-0" />
          <span className="truncate font-medium">{node.name}</span>
        </CollapsibleTrigger>
      </motion.div>
      <CollapsibleContent>
        {(node.children ?? []).map((c) => (
          <Node key={c.path} node={c} depth={depth + 1} selected={selected} onSelect={onSelect} />
        ))}
      </CollapsibleContent>
    </Collapsible>
  )
}

export function FileTree({ nodes, selected, onSelect, query, onQueryChange, className }: {
  nodes: FileNode[]
  selected: string
  onSelect: (path: string) => void
  query: string
  onQueryChange: (q: string) => void
  className?: string
}) {
  const shown = filterTree(nodes, query)
  return (
    <div className={cn("flex h-full min-h-0 flex-col", className)}>
      <div className="relative shrink-0 p-2">
        <Search className="text-muted-foreground pointer-events-none absolute top-1/2 left-4 size-4 -translate-y-1/2" />
        <Input
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          placeholder="Find a file"
          aria-label="Find a file"
          className="pl-8"
        />
      </div>
      <ScrollArea className="min-h-0 flex-1">
        <motion.div key={query} variants={stagger} initial="hidden" animate="show" className="flex flex-col px-2 pb-4">
          {shown.length === 0 ? (
            <p className="text-muted-foreground px-2 py-6 text-sm">No file matches <span className="font-mono">{query}</span>.</p>
          ) : (
            shown.map((n) => <Node key={n.path} node={n} depth={0} selected={selected} onSelect={onSelect} />)
          )}
        </motion.div>
      </ScrollArea>
    </div>
  )
}
