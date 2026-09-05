---
name: sandbox-port-in
invocation: model
description: Bring a named native GPUI screen into the surya browser sandbox with its Rust component and helper boundaries intact. Use for requests to bring or port a screen into the sandbox.
---

# Bring a screen into the sandbox

## Worked example: the Needs you heading and its row boundary

| Rust baseline | React destination |
|---|---|
| `inbox/needs_you.rs`: `NeedsYouPane::render`, heading `.flex().flex_row().items_baseline().gap(px(8.0)).pt(px(24.0)).pb(px(12.0))` | `screens/NeedsYou.tsx`: `NeedsYouPane`, `<div className="flex flex-row items-baseline gap-2 pt-6 pb-3">` |
| `inbox/needs_you/rows.rs`: `render_collapsed_row` | `screens/needs_you/rows.tsx`: `renderRow`, called below the heading |

Wrong: [NeedsYou.tsx](../../../sandbox/examples/port-in/wrong/NeedsYou.tsx), excerpt.

```tsx
export function NeedsYouPane({ rows, onOpenChat }: NeedsYouProps) {
  return (
    <section aria-labelledby="needs-you-heading" className="size-full overflow-y-scroll flex flex-col items-center px-12 pb-8">
      <div className="w-full max-w-184 min-w-0 flex flex-col">
        <div className="flex flex-row items-baseline gap-2 pt-6 pb-3">
          <h1 id="needs-you-heading" className="text-ui-20 font-semibold text-text">Needs you</h1>
          {rows.length > 0 && <span className="text-ui-13 text-text-faint">{rows.length} waiting</span>}
        </div>
        {rows.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-10 text-ui-13 text-text-faint">Nothing needs you.</div>
        ) : rows.map(row => (
          <button key={row.id} type="button" data-row={row.id} onClick={() => onOpenChat(row.chatId)}
            className="flex items-center gap-2 px-0.5 py-2.5 text-left">
            <span className="text-ui-10 text-warning">Question</span>
            <span className="flex-1 text-ui-13 text-text">{row.prompt}</span>
            <span className="text-ui-11 text-text-faint">answer below</span>
          </button>
        ))}
      </div>
    </section>
  );
}
```

Right: [NeedsYou.tsx](../../../sandbox/examples/port-in/right/NeedsYou.tsx), excerpt.

```tsx
export function NeedsYouPane({ rows, onOpenChat }: NeedsYouProps) {
  return (
    <section aria-labelledby="needs-you-heading" className="size-full overflow-y-scroll flex flex-col items-center px-12 pb-8">
      <div className="w-full max-w-184 min-w-0 flex flex-col">
        <div className="flex flex-row items-baseline gap-2 pt-6 pb-3">
          <h1 id="needs-you-heading" className="text-ui-20 font-semibold text-text">Needs you</h1>
          {rows.length > 0 && <span className="text-ui-13 text-text-faint">{rows.length} waiting</span>}
        </div>
        {rows.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-10 text-ui-13 text-text-faint">Nothing needs you.</div>
        ) : rows.map((row, index) => renderRow(row, index === 0, onOpenChat))}
      </div>
    </section>
  );
}
```

Inlining question rows beside the heading erases the `needs_you/rows.rs` boundary,
so the next row edit no longer points back to one native helper.

## Load and execute

Follow [GUIDE: Port in, per screen, from Rust to React](../../../sandbox/GUIDE.md#port-in-per-screen-from-rust-to-react).
Load [One-to-one layout and paint](../../../sandbox/GUIDE.md#one-to-one-layout-and-paint),
[Names and file boundaries](../../../sandbox/GUIDE.md#names-and-file-boundaries),
and [State and events](../../../sandbox/GUIDE.md#state-and-events) for the pair table,
fixture props and callback ownership; use [whitelist.json](../../../sandbox/whitelist.json)
as the admitted mapping. The example contract checks the component name and its
row-helper delegation; it does not infer arbitrary Rust boundaries automatically.

## Finish gate

From `sandbox/`, run in this order:

1. `npm run lint`
2. `npm test`
3. `npm run build`
4. Capture the screen beside its Windows reference frame, following
   [GUIDE: Reproduce proof](../../../sandbox/GUIDE.md#reproduce-proof).
