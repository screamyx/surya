---
name: sandbox-design
description: Iterate the look of a named surya sandbox screen with the owner in a browser, one requested change and comparison per round. Use for sandbox visual edits; finish by handing the change log to sandbox-port-back.
---

# Design in the browser

## Worked example: widen the question row's side padding

Wrong: [rows.tsx](../../../sandbox/examples/design/wrong/rows.tsx), excerpt.

```tsx
export function renderRow(row: InboxRow, first: boolean, onOpenChat: (id: string) => void) {
  return (
    <button key={row.id} type="button" onClick={() => onOpenChat(row.chatId)}
      aria-label={`Open chat: ${row.prompt}`} data-row={row.id} style={{ paddingLeft: 4, paddingRight: 4 }}
      className={first
        ? 'flex flex-row items-center gap-2 py-2.5 text-left cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
        : 'flex flex-row items-center gap-2 py-2.5 text-left border-t border-border cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
      {badge('Question', 'warning')}
      <span className="flex-1 min-w-0 truncate whitespace-nowrap text-ui-13 text-text">{row.prompt}</span>
      {hintChip('answer below')}
    </button>
  );
}
```

Right: [rows.tsx](../../../sandbox/examples/design/right/rows.tsx), excerpt.

```tsx
export function renderRow(row: InboxRow, first: boolean, onOpenChat: (id: string) => void) {
  return (
    <button key={row.id} type="button" onClick={() => onOpenChat(row.chatId)}
      aria-label={`Open chat: ${row.prompt}`} data-row={row.id}
      className={first
        ? 'flex flex-row items-center gap-2 px-1 py-2.5 text-left cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
        : 'flex flex-row items-center gap-2 px-1 py-2.5 text-left border-t border-border cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
      {badge('Question', 'warning')}
      <span className="flex-1 min-w-0 truncate whitespace-nowrap text-ui-13 text-text">{row.prompt}</span>
      {hintChip('answer below')}
    </button>
  );
}
```

Inline pixel padding bypasses the class-to-GPUI mapping, so the native padding
delta is no longer explicit.

## Load for the named screen

Read [GUIDE: Banned constructs and unsupported semantics](../../../sandbox/GUIDE.md#banned-constructs-and-unsupported-semantics),
[State and events](../../../sandbox/GUIDE.md#state-and-events),
[One-to-one layout and paint](../../../sandbox/GUIDE.md#one-to-one-layout-and-paint),
[Known gaps](../../../sandbox/GUIDE.md#known-gaps), and
[Reproduce proof](../../../sandbox/GUIDE.md#reproduce-proof).
Use [whitelist.json](../../../sandbox/whitelist.json) and the named screen's Windows
frames; the current Needs you frames are in `sandbox/proof/*-windows-reference.png`.

## Live design loop

1. Start: run `npm ci` in `sandbox/` if dependencies are missing, start
   `npm run dev -- --port 5177`, print the address, and open the named screen in
   the owner's browser with the reference fixture and theme. For Needs you,
   `http://127.0.0.1:5177/?theme=dark&state=seeded` matches the seeded dark reference.
2. For each request, make the smallest edit answering the owner's words, then run
   `npm run lint`, reload, and show the sandbox capture beside the Windows frame.
   Append one plain-words line to [CHANGES.md](../../../sandbox/CHANGES.md), naming
   the screen and the visible change. Describe it to the owner in one sentence
   without class names. Stop and wait for the next words; never batch rounds.
3. Refuse a class outside the whitelist and identify the GPUI method it would need,
   or state that no GPUI method exists; if the fork has a method excluded by the
   guide, name that method and the missing admitted mapping. Offer the nearest
   admitted effect and wait for the owner's choice. Never add CSS to bypass this.
4. On "done", run the finish gate below, capturing both light and dark at the
   reference size. Hand [CHANGES.md](../../../sandbox/CHANGES.md), the source diff
   and captures to [sandbox-port-back](../sandbox-port-back/SKILL.md) as the brief.

## Finish gate

From `sandbox/`, run in this order:

1. `npm run lint`
2. `npm test`
3. `npm run build`
4. Capture the screen beside its Windows reference frame, following
   [GUIDE: Reproduce proof](../../../sandbox/GUIDE.md#reproduce-proof).
