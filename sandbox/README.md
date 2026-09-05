# surya design sandbox

A browser sketchpad for iterating one native screen using the GPUI-compatible
subset in [GUIDE.md](GUIDE.md). The GPUI app is the product; this is not a second
frontend. Start with Needs you, switch light/dark and seeded/empty fixtures in the
preview controls, and hand the source diff back to an agent using the port guide.
Preview controls can be hidden with `?proof=1&theme=dark&state=seeded`.

From `sandbox/` in your own worktree (Node 22.12 or newer):

```sh
npm ci
npm run dev
npm run lint
```
