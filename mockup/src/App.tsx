import { BrowserRouter } from "react-router"
import { MotionConfig } from "motion/react"
import { AppRoutes } from "@/routes"

export default function App() {
  return (
    // reducedMotion="user" makes every variant in src/motion.ts obey the OS setting
    // without a single component checking for it. Content still arrives, it just
    // stops moving: motion never becomes the only way something is revealed.
    <MotionConfig reducedMotion="user">
      <BrowserRouter>
        <AppRoutes />
      </BrowserRouter>
    </MotionConfig>
  )
}
