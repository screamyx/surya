// Motion tokens. Durations and easings live here, next to the colour tokens in index.css.
// Restyle the feel of movement by changing these, never by editing a component.
//
// Decision 12 makes motion first class, and the owner's standing rule from 2026-08-22 is
// that overcooking beats undercooking. Reduced motion is handled globally by the
// MotionConfig in App.tsx, so no component needs its own media query.
import type { Transition, Variants } from "motion/react"

export const dur = { fast: 0.14, base: 0.22, slow: 0.36 } as const
export const ease = { out: [0.16, 1, 0.3, 1], inOut: [0.65, 0, 0.35, 1] } as const

export const spring: Transition = { type: "spring", stiffness: 420, damping: 34, mass: 0.8 }
// A heavier spring for a panel that carries real width: it should feel like furniture.
export const springPanel: Transition = { type: "spring", stiffness: 260, damping: 30, mass: 0.9 }
export const tween: Transition = { duration: dur.base, ease: ease.out }

// A list whose children arrive one after another (feed events, inbox rows, cards).
export const stagger: Variants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.04, delayChildren: 0.02 } },
}
export const rise: Variants = {
  hidden: { opacity: 0, y: 8 },
  show: { opacity: 1, y: 0, transition: tween },
}
export const fade: Variants = {
  hidden: { opacity: 0 },
  show: { opacity: 1, transition: { duration: dur.fast } },
}
// A panel or card that pops in (permission card, pin bubble, dialog body).
export const pop: Variants = {
  hidden: { opacity: 0, scale: 0.96 },
  show: { opacity: 1, scale: 1, transition: spring },
}
// A file or row that lights up when an agent touches it.
export const glow: Variants = {
  idle: { backgroundColor: "transparent" },
  touched: { backgroundColor: "var(--color-accent)", transition: { duration: dur.slow } },
}

// One pane replacing another in the same frame: the old one leaves before the new
// arrives, so nothing crosses over the top of anything.
export const swap: Variants = {
  hidden: { opacity: 0, y: 4 },
  show: { opacity: 1, y: 0, transition: { duration: dur.base, ease: ease.out } },
  exit: { opacity: 0, y: -4, transition: { duration: dur.fast, ease: ease.out } },
}

// A panel that opens and closes along one axis. Width is animated, not toggled, so the
// transcript grows into the space instead of jumping into it.
export const panel: Variants = {
  closed: { opacity: 0, width: 0, transition: { ...springPanel, opacity: { duration: dur.fast } } },
  open: { opacity: 1, width: "auto", transition: springPanel },
}

// A count that changed. One beat, not a loop: a number that pulses forever is noise.
export const beat: Variants = {
  rest: { scale: 1 },
  hit: { scale: [1, 1.18, 1], transition: { duration: dur.slow, ease: ease.inOut } },
}

// Something alive. This one does loop, because the thing it marks has not finished.
export const breathe: Variants = {
  rest: { opacity: 1 },
  live: { opacity: [1, 0.45, 1], transition: { duration: 1.8, repeat: Infinity, ease: ease.inOut } },
}

// A row entering or leaving a list that reorders itself. Pair with motion's `layout`
// prop so the neighbours slide rather than teleport.
export const row: Variants = {
  hidden: { opacity: 0, y: 6 },
  show: { opacity: 1, y: 0, transition: tween },
  exit: { opacity: 0, y: -6, height: 0, transition: { duration: dur.fast } },
}
export const layoutSpring: Transition = { type: "spring", stiffness: 320, damping: 32 }
