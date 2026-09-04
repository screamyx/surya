# surya/mockup - static mockup of every surya 1.0 RC surface

Ground truth for builders of the real app. Stock shadcn ("base-nova" preset, neutral), Tailwind v4, React 19, react-router 7, motion, CodeMirror 6. Runs on Bun.

## Rules
- Every file 500 lines max. Split screens into `src/screens/<name>/` parts when they grow.
- Stock shadcn components from `src/components/ui/` only. Never hand-roll a primitive that shadcn has. Add a missing one with `bunx --bun shadcn@latest add <name>`.
- No hard-coded colour, radius, or spacing. Tokens only (`bg-primary`, `text-muted-foreground`, `rounded-lg`, `--radius`). A restyle must be a token change.
- Motion through `motion/react` with the tokens in `src/motion.ts`. No CSS keyframes of your own, no hand-rolled transitions.
- Phone first. Every screen works at 390px and at 1440px. The shell already gives a bottom nav under `md` and a sidebar above it.
- Data comes from `src/data.ts`. Never edit it. Never edit `src/routes.tsx`, `src/components/app-shell.tsx`, `src/components/status.tsx`, `src/motion.ts`.
- Static means static: no fetch, no timers that change data, no state beyond what a click needs to show (open a dialog, switch a tab, pick a file).
- Components use the base-ui `render` prop, not `asChild`: `<Button render={<Link to="/x" />}>`.

## Run
```
bun install
bun run dev          # :4790 on 0.0.0.0
bun run build        # must pass before you report done
```

## Surfaces
See `surfaces` in `src/routes.tsx`. One screen file per surface under `src/screens/`.
