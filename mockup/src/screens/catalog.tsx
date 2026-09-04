// Screen: catalog. Feature 6, the A2UI card catalog. Every shape an agent answer can take.
import { motion } from "motion/react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { cards } from "@/data"
import { rise, stagger } from "@/motion"
import type { CardSample } from "@/data"
import { A2UICard } from "@/screens/catalog/cards"

// data.ts names the photo shape after the first thing it was sampled with. The catalog
// shows what the shape actually is.
const shapeName = (card: CardSample) => ("photo" in card ? "record" : card.type)

const steps = [
  "The agent answers with a small piece of JSON, not with a web page.",
  "surya checks that JSON against this catalog and drops anything that is not in it.",
  "The card renders with your theme, so it matches the rest of the app.",
]

export function CatalogScreen() {
  return (
    <div className="mx-auto w-full max-w-7xl px-4 py-6 md:px-6 md:py-8">
      <header className="flex flex-col gap-1">
        <h1 className="font-heading text-2xl font-semibold tracking-tight md:text-3xl">Cards agents can show you</h1>
        <p className="text-muted-foreground text-sm">Agents pick from this catalog. They cannot draw anything else.</p>
        <p className="text-muted-foreground text-sm">Six built-in shapes. A workspace can add its own cards.</p>
      </header>

      <motion.div
        variants={stagger}
        initial="hidden"
        animate="show"
        className="mt-6 grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3"
      >
        {cards.map((card) => (
          <motion.div key={card.type} variants={rise} className="min-w-0">
            <Card className="h-full">
              <CardHeader>
                <CardTitle className="text-muted-foreground font-mono text-xs font-normal">{shapeName(card)}</CardTitle>
              </CardHeader>
              <CardContent className="min-w-0">
                <A2UICard card={card} />
              </CardContent>
            </Card>
          </motion.div>
        ))}
      </motion.div>

      <Card className="mt-6">
        <CardHeader>
          <CardTitle>How a card gets here</CardTitle>
        </CardHeader>
        <CardContent>
          <ol className="flex flex-col gap-3">
            {steps.map((step, i) => (
              <li key={step} className="flex gap-3">
                <span className="bg-muted text-muted-foreground flex size-6 shrink-0 items-center justify-center rounded-full text-xs font-medium tabular-nums">{i + 1}</span>
                <span className="text-sm leading-6">{step}</span>
              </li>
            ))}
          </ol>
        </CardContent>
      </Card>
    </div>
  )
}
