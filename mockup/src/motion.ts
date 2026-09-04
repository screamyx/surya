// Motion tokens. Durations and easings live here, next to the colour tokens in index.css.
// Restyle the feel of movement by changing these, never by editing a component.
import type { Transition, Variants } from "motion/react"

export const dur = { fast: 0.15, base: 0.25, slow: 0.4 } as const
export const ease = { out: [0.16, 1, 0.3, 1], inOut: [0.65, 0, 0.35, 1] } as const

export const spring: Transition = { type: "spring", stiffness: 420, damping: 34, mass: 0.8 }
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
