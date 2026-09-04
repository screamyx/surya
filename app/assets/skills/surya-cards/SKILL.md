---
name: surya-cards
description: "Write a card for surya to draw instead of answering in prose. Use when the answer is a record, a list, a form, a yes/no decision, a set of file changes, or a number - or when show_card returns an error and you need the exact shape. Not for ordinary prose answers."
---

# surya cards

`show_card` puts a native card on the user's screen. Pass one of the six
built-in shapes below, or raw A2UI v0.9.1 messages when a shape does not fit.

Call `list_cards` first in a workspace you have not shown a card in: it returns
these six plus the cards that workspace defines in `.surya/cards/*.json`.

Keep the reply text to one line when you show a card.

## The six shapes

Every shape takes `title`, and every shape takes an optional `actions` list:

```json
"actions": [{ "label": "Open", "event": "open_thing", "context": { "id": 412 } }]
```

The first action is drawn as the primary button. `event` and `context` come
back to the app when the user taps it.

### record - one thing and its fields

```json
{
  "shape": "record",
  "title": "2021 Toyota Alphard 2.5 SC",
  "subtitle": "KSS-0412 - 34 days in stock",
  "image": "https://example.test/photo.jpg",
  "badges": ["In stock"],
  "fields": [
    { "label": "Price", "value": "RM 268,800" },
    { "label": "Days", "value": 34 }
  ]
}
```

### table - a list to scan

```json
{
  "shape": "table",
  "title": "Leads with no follow-up date",
  "columns": ["Lead", "Car", "Came in", "Salesperson"],
  "rows": [
    ["Ahmad F.", "Alphard SC", "2 days ago", "Zul"],
    ["Mei Ling", "Vellfire ZG", "4 days ago", "Farah"]
  ]
}
```

### form - something for the user to fill in

Field `type` is `text`, `select` or `date`. `key` names the slot in the data
model; leave it out and the fields become `field1`, `field2` and so on.

```json
{
  "shape": "form",
  "title": "Set the follow-up",
  "fields": [
    { "key": "lead", "label": "Lead", "type": "text", "value": "Ahmad F." },
    { "key": "when", "label": "When", "type": "date", "value": "2026-09-06" },
    { "key": "who", "label": "Salesperson", "type": "select", "value": "Zul",
      "options": ["Zul", "Farah", "Hafiz"] }
  ],
  "submit": "Save follow-up",
  "event": "save_follow_up"
}
```

### approval - a yes or no before you act

```json
{
  "shape": "approval",
  "title": "Write 3 leads",
  "summary": "Set next_follow_up_at to tomorrow 08:00 for the three leads above.",
  "code": "Lead::whereIn('id', [412, 418, 421])\n    ->update(['next_follow_up_at' => now()->addDay()]);",
  "approve": "Write them",
  "reject": "Not yet",
  "context": { "ids": [412, 418, 421] }
}
```

The buttons send `surya_approve` and `surya_reject` with your `context`.

### diff-summary - what changed

```json
{
  "shape": "diff-summary",
  "title": "What changed",
  "files": [
    { "path": "app/Models/Lead.php", "added": 12, "removed": 0 },
    { "path": "database/migrations/2026_09_05_000001_add_follow_up.php", "added": 28, "removed": 0 }
  ],
  "note": "Two new columns, one job, no route changes."
}
```

### metric - one number

`series` is optional. It rides the card's data model, so a renderer that can
draw a sparkline does; one that cannot shows the numbers.

```json
{
  "shape": "metric",
  "title": "Leads followed up this week",
  "value": "38",
  "delta": "+12 vs last week",
  "series": [4, 6, 3, 8, 7, 5, 5]
}
```

## When a shape does not fit

Pass A2UI v0.9.1 messages instead of a short-form card, either one message or a
list of them. They go through untouched.

An A2UI surface is a flat list of components linked by id. One component must
have the id `root`. Containers name their children by id; children are never
nested inline.

```json
{ "card": [
  { "version": "v0.9.1",
    "createSurface": {
      "surfaceId": "release_1",
      "catalogId": "https://a2ui.org/specification/v0_9/catalogs/basic/catalog.json"
    }},
  { "version": "v0.9.1",
    "updateComponents": { "surfaceId": "release_1", "components": [
      { "id": "root", "component": "Card", "child": "body" },
      { "id": "body", "component": "Column", "children": ["head", "note"] },
      { "id": "head", "component": "Text", "text": "Release 0.2.34", "variant": "h3" },
      { "id": "note", "component": "Text", "text": "Built from **v2**.", "variant": "body" }
    ]}}
]}
```

Components in the basic catalog: `Text`, `Image`, `Icon`, `Video`,
`AudioPlayer`, `Row`, `Column`, `List`, `Card`, `Tabs`, `Divider`, `Modal`,
`Button`, `CheckBox`, `TextField`, `DateTimeInput`, `ChoicePicker`, `Slider`.

Bind a value to the data model with `{"path": "/some/key"}` instead of a
literal, then send `updateDataModel` with `path` and `value` to change it
without resending the components.

Full spec: A2UI v0.9.1, `specification/v0_9_1/docs/a2ui_protocol.md` in the
a2ui-project/a2ui repository.

## Cards this workspace brings

A workspace defines its own cards as JSON in `.surya/cards/`, one file per
card, each carrying `name`, `description` and a layout over the same
primitives. They are data, not code, so you can add one the way you add a
skill. `list_cards` reports the ones that exist here.
