# Browser.Watch hold (E2E-ENGINE-01), Linux :7, 2026-09-06

Two apps on one headless engine (mock harness), same rig before and after.
Before is main 9c430470; after is fix/browser-watch-hold 41405781.
The clock starts once the engine has accepted both apps.

| Run | replaced (engine) | app A attached | app B attached |
|---|---|---|---|
| before, two apps, no Browser tab, 90 s | 17 | 9 | 9 |
| before, pre-rename a538456, same, 90 s | 28 | 23 | 23 |
| before, one app alone, 180 s | 0 | 1 | - |
| after, two apps, no Browser tab, 90 s | 0 | 0 | 0 |
| after, two apps, B shows its Browser tab, 90 s | 0 | 0 | 1 |
| after, one app, chat-only 600 s (a mock run, auto-titled at t=22 s) | 0 | 0 | - |

"replaced" counts the engine line "browser broker: a new app pane replaced the previous one".
"attached" counts the app's stdout line "browser-agent: attached to the engine's Browser.Watch".
In the 600 s run the engine log has no line containing "browser" at all.

Rig: `SURYA_HARNESS=mock surya headless` with `SURYA_IPC_TOKEN`, then one or two `surya` apps dialed to its port with their own data dirs, stdout captured; the B-shows-its-tab case sets `SURYA_OPEN_PANE=browser` on B.
