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

- [sandbox-port-in](../.claude/skills/sandbox-port-in/SKILL.md): bring a native screen into the sandbox.
- [sandbox-design](../.claude/skills/sandbox-design/SKILL.md): iterate in the browser, one change per round, recorded in CHANGES.md.
- [sandbox-port-back](../.claude/skills/sandbox-port-back/SKILL.md): carry the accepted screen diff and CHANGES.md back into GPUI.

[Annotate](annotate/README.md): tap a screen element to send the design agent a comment with its exact source line.
