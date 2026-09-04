// The cards a workspace brings with it, read from its own `.surya/cards` folder.
// surya ships the six shapes above; everything here is the repo's, so the names are the
// workspace's names and mean nothing to surya.
import { motion } from "motion/react"
import { Pencil, Plus } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { workspaceCards, workspaces } from "@/data"
import { rise, stagger } from "@/motion"
import { A2UICard } from "@/screens/catalog/cards"
import { shapeName } from "@/screens/catalog/shape"

type WorkspaceCard = (typeof workspaceCards)[string][number]

function SectionHeader({ name, count }: { name: string; count: number }) {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
      <span className="bg-muted text-foreground grid size-6 shrink-0 place-items-center rounded-md text-xs font-semibold uppercase">
        {name.slice(0, 2)}
      </span>
      <h2 className="font-heading text-lg font-semibold tracking-tight">{name}</h2>
      <Badge variant="secondary">{count}</Badge>
      <div className="ml-auto">
        <Tooltip>
          <TooltipTrigger
            render={
              <Button variant="outline" size="sm">
                <Plus data-icon="inline-start" />
                New card
              </Button>
            }
          />
          <TooltipContent>Describe it to an agent, it writes the JSON</TooltipContent>
        </Tooltip>
      </div>
    </div>
  )
}

function WorkspaceCardTile({ card }: { card: WorkspaceCard }) {
  return (
    <Card className="h-full">
      <CardHeader>
        <CardTitle className="font-mono text-sm">{card.name}</CardTitle>
        <CardDescription>{card.description}</CardDescription>
        <CardAction>
          <Badge variant="outline" className="font-mono text-xs font-normal">shape: {shapeName(card.sample)}</Badge>
        </CardAction>
        <p className="text-muted-foreground mt-1 truncate font-mono text-xs" title={card.file}>{card.file}</p>
      </CardHeader>
      <CardContent className="min-w-0 flex-1">
        <A2UICard card={card.sample} />
      </CardContent>
      <CardFooter>
        <Button variant="ghost" size="sm" className="text-muted-foreground">
          <Pencil data-icon="inline-start" />
          Edit card
        </Button>
      </CardFooter>
    </Card>
  )
}

export function WorkspaceCardSections() {
  const sections = workspaces.filter((w) => (workspaceCards[w.id] ?? []).length > 0)
  return (
    <>
      {sections.map((w) => {
        const list = workspaceCards[w.id]
        return (
          <section key={w.id} className="mt-8">
            <SectionHeader name={w.name} count={list.length} />
            <p className="text-muted-foreground mt-1 text-sm">
              From this workspace's .surya/cards folder. Any agent working here can add one.
            </p>
            <motion.div
              variants={stagger}
              initial="hidden"
              animate="show"
              className="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3"
            >
              {list.map((card) => (
                <motion.div key={card.name} variants={rise} className="min-w-0">
                  <WorkspaceCardTile card={card} />
                </motion.div>
              ))}
            </motion.div>
          </section>
        )
      })}
    </>
  )
}
