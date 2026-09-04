// A number that beats once when it changes, used by more than one screen.
// The beat is decoration on top of text that is always rendered, so with motion off
// the count still reads: nothing here is revealed by the movement.
import { useEffect, useRef, useState } from "react"
import { motion } from "motion/react"
import { beat } from "@/motion"
import { cn } from "@/lib/utils"

export function BeatCount({ value, className }: { value: number; className?: string }) {
  const [hit, setHit] = useState(false)
  const seen = useRef(value)

  useEffect(() => {
    if (seen.current === value) return
    seen.current = value
    setHit(true)
  }, [value])

  return (
    <motion.span
      variants={beat}
      initial="rest"
      animate={hit ? "hit" : "rest"}
      onAnimationComplete={() => setHit(false)}
      className={cn("tnum inline-block", className)}
    >
      {value}
    </motion.span>
  )
}
