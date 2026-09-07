# Agent mail

What decision 19 asked for, and where it lives in the engine.

## The rule

One table. One delivery id per message. Delivery injects the message into the
recipient's next turn. Ack is automatic when that turn ends.

```
sender ──Mail.Send──▶ mail table ──▶ delivery pump ──dispatch──▶ recipient's turn
                          ▲                                            │
                          └──────────── ack on turn end ◀──────────────┘
```

## Addresses

| You write | It resolves to |
| --- | --- |
| `chat-b` | the chat with that id |
| `Front desk` | the chat whose title is that alias |
| `#space-mail` | every non-archived chat in that space, one row each |
| anything unknown | itself, queued until an agent claims the id |

An unknown address never fails a send. That is agb's reserve-on-spawn: a brief
can mail an agent a moment before it exists.

## Delivery

A send tries immediately. The pump retries whenever any agent settles.

Both go through the same call, `dispatch`, which folds the text into a live
steerable run's mailbox at its next step boundary and starts a fresh run
otherwise. It returns the run id, which is what the ack is keyed on.

The recipient reads one line per message, in send order:

```
[MAIL mail-6a1f… from chat-a] please check the diff
```

An agent with no chat row on this engine keeps its mail queued. So does a row
whose `to_device` is not this device - the forwarding call is the only thing
missing for cross-server mail, not the schema.

## How the row looks

The delivered row is a user entry, because that is what the harness has to
read it as. It also carries a `source` field naming the senders, and that
field is the only thing the app looks at to decide the row is mail. Text is
never consulted: a prompt the owner pastes can say `[MAIL ...]` and still
renders as his own (surya#198).

The envelope text stays in the row exactly as delivered. The harness needs the
ids it has to ack, and a crash-recovery re-dispatch resends the stored prompt
verbatim, so the brackets cannot be stripped on the way in. The app strips
them on the way out instead: it lifts each envelope back out at render time
and draws one left-hand row per sender, named, opposite the owner's own
right-hand bubble.

The grammar has one definition, `app/crates/proto/src/mail.rs`, because both
sides use it - the engine writes the block, the app reads it back.

`SURYA_DEMO_MAIL=<sender>|<body>` seeds a chat with one typed row and one
mailed-in row, for a screenshot. A real mail row needs a second agent and a
shot taken before its turn answers.

## Table

`mail.sqlite3`, a sibling of `docs.sqlite3` in the same profile root, so mail
inherits the local/synced boundary.

| Column | Means |
| --- | --- |
| `id` | the delivery id |
| `sender`, `recipient` | the addresses as written |
| `to_agent` | the resolved agent this row is for |
| `body` | the message |
| `created_at`, `delivered_at`, `acked_at` | the three states, as timestamps |
| `from_device`, `to_device` | day-one shape for cross-server forwarding |
| `run_id` | the turn that carried it |

## Surfaces

| Call | Does |
| --- | --- |
| `Mail.Send {from, to, body, toDevice?}` | `{ids, recipients}` |
| `Mail.List {agent?}` | mail for one agent, or the recent feed |
| `Mail.Ack {id}` | the manual "seen" |
| `WatchMail` | current list, then one item per change |

Ingress from the surya-mcp seat, both live at once:

- `$XDG_RUNTIME_DIR/surya/mail.sock` - one JSON object per line, replies with
  the receipt;
- `~/.surya/mail.jsonl` - appended records, tailed from the end.

Record: `{"from":"…","to":"…","body":"…","toDevice":"…"?}`.

CLI shim, so an agb-shaped skill can alias to it:

```
surya mail send <to> <body> [--from X]
surya mail drain [--agent X]
surya mail ack <id>
```

## Not here

The Messages pane (a later seat reads `WatchMail`), forwarding between
devices, and auth.

## Files

| File | Lines | Holds |
| --- | --- | --- |
| `app/crates/engine/src/mail/mod.rs` | 396 | the service: send, list, ack, address resolution |
| `app/crates/engine/src/mail/store.rs` | 426 | the table |
| `app/crates/engine/src/mail/delivery.rs` | 306 | the pump and the ack |
| `app/crates/engine/src/mail/envelope.rs` | 268 | the row, the address, the state |
| `app/crates/engine/src/mail/origin.rs` | 75 | who a dispatched turn writes its row as |
| `app/crates/proto/src/mail.rs` | 214 | the envelope grammar, and the row's `source` field |
| `app/crates/ui/src/transcript/mail_row.rs` | 268 | the row the owner sees |
| `app/crates/ui/src/transcript/demo_mail.rs` | 139 | `SURYA_DEMO_MAIL`, for the shot |
| `app/crates/engine/src/mail/ingress.rs` | 361 | socket and jsonl ingress |
| `app/crates/engine/src/mail/rpc.rs` | 120 | the four calls |
| `app/apps/surya/src/mail_cli.rs` | 119 | the CLI shim |
| `app/crates/engine/tests/agent_mail.rs` | 185 | the proof |

## Proof

`cargo test -p surya-engine --test agent_mail`, 2 passed 0 failed, 2026-09-05:

```
unknown recipient: sent=1 delivered=0 acked=0
proof 1: sent=1 delivered=1 acked=1
proof 2: sent=1 recipients=2 ids=2
proof 2: sent=1 delivered=2
```

Proof 1 asserts B's transcript carries the literal
`[MAIL <id> from chat-a] please check the diff` as a user entry, that the row
records the run that carried it, and that the ack lands only after that turn
ends. Proof 2 asserts each of the two agents carries its own copy with its own
delivery id.
