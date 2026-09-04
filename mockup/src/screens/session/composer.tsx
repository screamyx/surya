// The composer, sticky at the bottom of the transcript. Its text lives in the session's
// compose bridge, so the preview pane can fill it from a pin without leaving the screen.
import { useEffect, useRef } from "react"
import { ArrowUp } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupTextarea } from "@/components/ui/input-group"
import { Command, CommandEmpty, CommandGroup, CommandItem, CommandList } from "@/components/ui/command"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { SlashPalette } from "@/screens/session/slash"
import { useCompose } from "@/screens/session/compose"
import { modelName } from "@/screens/session/model-sheet"

export function Composer({ model, onModel }: { model: string; onModel: () => void }) {
  const { draft, setDraft, handed, taken } = useCompose()
  const ref = useRef<HTMLTextAreaElement>(null)
  // A pin sent from the preview lands here, so put the cursor where the person will type.
  useEffect(() => {
    if (!handed) return
    ref.current?.focus()
    ref.current?.setSelectionRange(draft.length, draft.length)
    taken()
  }, [handed, draft, taken])
  const open = draft.startsWith("/") && !draft.includes(" ")
  const pick = (name: string) => { setDraft(`/${name} `); ref.current?.focus() }
  // A menu command never becomes text. surya draws the sheet and sends the line for you.
  const menu = () => { setDraft(""); onModel() }
  return (
    <div className="bg-background sticky bottom-0 z-20 shrink-0 border-t px-4 pt-3 pb-3">
      <div className="relative">
        {open && (
          <SlashPalette
            query={draft.slice(1)}
            onPick={pick}
            onMenu={menu}
            Command={Command}
            CommandList={CommandList}
            CommandGroup={CommandGroup}
            CommandItem={CommandItem}
            CommandEmpty={CommandEmpty}
          />
        )}
        <InputGroup>
          <InputGroupTextarea
            ref={ref}
            name="reply"
            rows={2}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            placeholder="Reply, ask for the next thing, or type / for a command"
          />
          <InputGroupAddon align="block-end" className="border-t">
            <Badge
              variant="outline"
              className="hover:bg-muted cursor-pointer font-mono"
              render={<button type="button" onClick={onModel} aria-label="Change the model" />}
            >
              {modelName(model)}
            </Badge>
            <span className="text-muted-foreground hidden items-center gap-1.5 text-xs sm:flex">
              <KbdGroup><Kbd>⌘</Kbd><Kbd>↵</Kbd></KbdGroup> to send · <Kbd>/</Kbd> commands
            </span>
            <InputGroupButton variant="default" size="icon-sm" className="ml-auto" aria-label="Send">
              <ArrowUp />
            </InputGroupButton>
          </InputGroupAddon>
        </InputGroup>
      </div>
    </div>
  )
}
