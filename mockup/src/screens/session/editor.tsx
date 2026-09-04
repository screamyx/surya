// The editor pane: open tabs, the real CodeMirror, and a side-by-side diff of what the
// agent changed. Desktop only - decision 20 point 4 cuts the editor and the tree from the
// phone, where the files pane shows changed files and the diff instead.
import { Fragment, useEffect, useRef } from "react"
import CodeMirror from "@uiw/react-codemirror"
import { javascript } from "@codemirror/lang-javascript"
import { markdown } from "@codemirror/lang-markdown"
import { EditorView, lineNumbers } from "@codemirror/view"
import { EditorState } from "@codemirror/state"
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language"
import { tags as t } from "@lezer/highlight"
import { Prec } from "@codemirror/state"
import { MergeView } from "@codemirror/merge"
import { Badge } from "@/components/ui/badge"
import { Breadcrumb, BreadcrumbItem, BreadcrumbList, BreadcrumbPage, BreadcrumbSeparator } from "@/components/ui/breadcrumb"
import { Button } from "@/components/ui/button"
import { Separator } from "@/components/ui/separator"
import { Spinner } from "@/components/ui/spinner"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { StatusDot } from "@/components/status"
import { fileContents, leadPhpAfter, leadPhpBefore } from "@/data"

export const DIFF_PATH = "backend/app/Models/Lead.php"

// The merge view paints its own surface, so it has to be told our tokens or it
// inherits page colours and lands ivory-on-white in dark mode.
const surface = Prec.highest(EditorView.theme({
  "&": { backgroundColor: "var(--canvas)", color: "var(--foreground)" },
  ".cm-content": { caretColor: "var(--primary)" },
  ".cm-gutters": { backgroundColor: "var(--canvas)", color: "var(--muted-foreground)", borderRight: "1px solid var(--border)" },
  ".cm-lineNumbers .cm-gutterElement": { color: "var(--muted-foreground)" },
  ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "transparent" },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": { backgroundColor: "var(--accent)" },
}))

// One syntax palette, written in tokens, so the editor swaps with the theme like
// everything else. Prec.highest keeps basicSetup's stock style from winning.
const codeHighlight = HighlightStyle.define([
  { tag: [t.keyword, t.controlKeyword, t.modifier, t.operatorKeyword, t.definitionKeyword, t.moduleKeyword], color: "var(--code-key)" },
  { tag: [t.string, t.special(t.string), t.regexp], color: "var(--code-str)" },
  { tag: [t.number, t.bool, t.null, t.atom], color: "var(--code-num)" },
  { tag: [t.comment, t.lineComment, t.blockComment, t.meta], color: "var(--code-com)", fontStyle: "italic" },
  { tag: [t.function(t.variableName), t.definition(t.variableName), t.className, t.typeName, t.propertyName, t.attributeName, t.variableName, t.punctuation, t.bracket, t.operator, t.heading, t.link], color: "var(--code-txt)" },
])

const codeExtensions = () => [surface, Prec.highest(syntaxHighlighting(codeHighlight))]

// What each side of the merge view needs: read only, line numbers, PHP-ish colours.
const diffSide = () => [
  javascript(),
  lineNumbers(),
  ...codeExtensions(),
  EditorView.editable.of(false),
  EditorState.readOnly.of(true),
]

const langFor = (path: string) => (path.endsWith(".md") ? markdown() : javascript())
const nameOf = (path: string) => path.split("/").pop() ?? path
const labelFor = (path: string) => (path.endsWith(".md") ? "Markdown" : "PHP")

function SideBySideDiff() {
  const host = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const parent = host.current
    if (!parent) return
    const view = new MergeView({
      parent,
      a: { doc: leadPhpBefore, extensions: diffSide() },
      b: { doc: leadPhpAfter, extensions: diffSide() },
      gutter: true,
      highlightChanges: true,
    })
    return () => view.destroy()
  }, [])
  return (
    <div className="flex h-full flex-col">
      <div className="text-muted-foreground bg-muted/50 grid shrink-0 grid-cols-2 border-b text-xs">
        <span className="border-r px-3 py-1.5">Before</span>
        <span className="px-3 py-1.5">After, what raven changed</span>
      </div>
      <div ref={host} className="min-h-0 flex-1 text-sm [&_.cm-editor]:h-full! [&_.cm-mergeView]:h-full [&_.cm-mergeViewEditors]:h-full" />
    </div>
  )
}


function DiffBody({ path }: { path: string }) {
  if (path !== DIFF_PATH) {
    return (
      <div className="flex min-h-64 flex-col items-center justify-center gap-1 p-8 text-center">
        <p className="text-sm font-medium">No agent changes in this file</p>
        <p className="text-muted-foreground text-sm">raven only edited <span className="font-mono">Lead.php</span> in this run.</p>
      </div>
    )
  }
  return <SideBySideDiff />
}

function EditBody({ path }: { path: string }) {
  return (
    <CodeMirror
      key={path}
      value={fileContents[path] ?? ""}
      theme="none"
      height="100%"
      className="h-full [&_.cm-editor]:h-full! [&_.cm-scroller]:h-full"
      editable={false}
      basicSetup={{ lineNumbers: true, foldGutter: false, highlightActiveLine: false, highlightActiveLineGutter: false }}
      extensions={[langFor(path), ...codeExtensions()]}
    />
  )
}

export function FileEditor({ path, tabs, onSelectTab, mode, onModeChange }: {
  path: string
  tabs: string[]
  onSelectTab: (path: string) => void
  mode: "edit" | "diff"
  onModeChange: (mode: "edit" | "diff") => void
}) {
  const parts = path.split("/")
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="text-muted-foreground flex shrink-0 items-center gap-2 border-b px-3 py-2 text-sm">
        <Spinner className="size-3.5 shrink-0" />
        <span className="truncate">raven is writing tests in <span className="font-mono">backend/tests/Feature/FollowUpReminderTest.php</span></span>
      </div>

      <div className="flex shrink-0 items-center gap-2 border-b px-2 py-2">
        <div className="min-w-0 flex-1 overflow-x-auto">
          <Tabs value={path} onValueChange={(v) => onSelectTab(v as string)}>
            <TabsList variant="line" className="h-8">
              {tabs.map((t) => (
                <TabsTrigger key={t} value={t} className="shrink-0 px-2">{nameOf(t)}</TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
        </div>
        <ToggleGroup
          value={[mode]}
          onValueChange={(v) => v.length && onModeChange(v[0] as "edit" | "diff")}
          variant="outline"
          size="sm"
          spacing={0}
          className="shrink-0"
        >
          {/* "Changes", not "Diff": the pane's own Diff tab reviews every changed file,
              and this one only shows what moved inside the file you have open. */}
          <ToggleGroupItem value="edit">Edit</ToggleGroupItem>
          <ToggleGroupItem value="diff">Changes</ToggleGroupItem>
        </ToggleGroup>
      </div>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden p-3">
        <Breadcrumb className="mb-2 shrink-0">
          <BreadcrumbList className="font-mono text-xs">
            {parts.map((p, i) => (
              <Fragment key={`${p}-${i}`}>
                {i > 0 && <BreadcrumbSeparator />}
                <BreadcrumbItem>{i === parts.length - 1 ? <BreadcrumbPage>{p}</BreadcrumbPage> : p}</BreadcrumbItem>
              </Fragment>
            ))}
          </BreadcrumbList>
        </Breadcrumb>
        <div className="relative min-h-0 flex-1 overflow-hidden rounded-lg border">
          <div className="absolute inset-0 overflow-auto">
            {mode === "edit" ? <EditBody path={path} /> : <DiffBody path={path} />}
          </div>
        </div>
      </div>

      <div className="text-muted-foreground flex shrink-0 flex-wrap items-center gap-x-3 gap-y-2 border-t px-3 py-2 text-xs">
        <span className="flex items-center gap-1.5"><StatusDot status="working" />raven edited this 2 min ago</span>
        <Separator orientation="vertical" className="hidden h-3 sm:block" />
        <span className="tabular-nums">Ln 12, Col 8</span>
        <Separator orientation="vertical" className="hidden h-3 sm:block" />
        <Badge variant="outline">{labelFor(path)}</Badge>
        <div className="ml-auto flex items-center gap-2">
          <Button size="sm" variant="ghost" className="h-7 px-2.5">Revert</Button>
          <Button size="sm" className="h-7 px-2.5">Save</Button>
        </div>
      </div>
    </div>
  )
}
