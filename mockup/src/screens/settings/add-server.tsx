// Add server: pick or type a host, run the checks, install over SSH, done.
// Four steps in one sheet, so the whole thing happens without leaving Settings.
import { useState } from "react"
import { AnimatePresence, motion } from "motion/react"
import { ArrowLeft, ArrowRight, Check } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from "@/components/ui/sheet"
import { useIsMobile } from "@/hooks/use-mobile"
import { StepChecks, StepDone, StepInstall, StepPickMachine } from "@/screens/settings/add-server-steps"
import { rise } from "@/motion"
import { cn } from "@/lib/utils"

const steps = ["Pick a machine", "Checks", "Install", "Done"]

function VerticalStepper({ step }: { step: number }) {
  return (
    <ol className="flex flex-col">
      {steps.map((label, i) => (
        <li key={label} className={cn("flex gap-2.5", i < steps.length - 1 && "min-h-14")}>
          <div className="flex flex-col items-center">
            <span
              className={cn(
                "grid size-6 shrink-0 place-items-center rounded-full border text-xs font-medium tabular-nums",
                i < step && "bg-primary text-primary-foreground border-transparent",
                i === step && "border-primary text-foreground border-2",
                i > step && "text-muted-foreground"
              )}
            >
              {i < step ? <Check className="size-3.5" /> : i + 1}
            </span>
            {i < steps.length - 1 && <span className={cn("my-1 w-px flex-1", i < step ? "bg-primary" : "bg-border")} />}
          </div>
          <span className={cn("pt-0.5 text-sm", i === step ? "font-medium" : "text-muted-foreground")}>{label}</span>
        </li>
      ))}
    </ol>
  )
}

function TopProgress({ step }: { step: number }) {
  return (
    <div className="flex flex-col gap-2 border-b p-4 md:hidden">
      <div className="flex gap-1.5">
        {steps.map((label, i) => (
          <span key={label} className={cn("h-1 flex-1 rounded-full", i <= step ? "bg-primary" : "bg-muted")} />
        ))}
      </div>
      <span className="text-muted-foreground text-xs">
        Step {step + 1} of {steps.length} · {steps[step]}
      </span>
    </div>
  )
}

export function AddServerSheet({ open, onOpenChange }: { open: boolean; onOpenChange: (v: boolean) => void }) {
  const isMobile = useIsMobile()
  const [step, setStep] = useState(0)
  const [host, setHost] = useState("")
  const [loggedIn, setLoggedIn] = useState(false)

  // Reopening starts a fresh machine, not the last one half-installed.
  const change = (v: boolean) => {
    onOpenChange(v)
    if (!v) {
      setStep(0)
      setHost("")
      setLoggedIn(false)
    }
  }

  const canContinue = step === 0 ? host.trim().length > 0 : step === 1 ? loggedIn : true

  return (
    <Sheet open={open} onOpenChange={change}>
      <SheetContent
        side={isMobile ? "bottom" : "right"}
        className={cn("gap-0 p-0", isMobile ? "data-[side=bottom]:h-svh" : "data-[side=right]:sm:max-w-2xl")}
      >
        <SheetHeader className="border-b">
          <SheetTitle>Add a server</SheetTitle>
          <SheetDescription>Any machine you can reach. surya installs its daemon there over SSH.</SheetDescription>
        </SheetHeader>

        <TopProgress step={step} />

        <div className="flex min-h-0 flex-1 flex-col md:flex-row">
          <nav className="hidden shrink-0 border-r p-3 md:block md:w-52">
            <VerticalStepper step={step} />
          </nav>

          <div className="min-h-0 flex-1 overflow-y-auto p-4">
            <AnimatePresence mode="wait" initial={false}>
              <motion.div key={step} variants={rise} initial="hidden" animate="show" exit="hidden" className={cn("min-h-full", step === 3 && "flex items-center justify-center")}>
                {step === 0 && <StepPickMachine host={host} setHost={setHost} />}
                {step === 1 && <StepChecks host={host} loggedIn={loggedIn} onLogin={() => setLoggedIn(true)} />}
                {step === 2 && <StepInstall host={host} />}
                {step === 3 && <StepDone host={host} onClose={() => change(false)} />}
              </motion.div>
            </AnimatePresence>
          </div>
        </div>

        {step < 3 && (
          <SheetFooter className="flex-row items-center justify-between border-t">
            <Button variant="ghost" className="h-11 md:h-8" disabled={step === 0} onClick={() => setStep((s) => s - 1)}>
              <ArrowLeft data-icon="inline-start" />Back
            </Button>
            <Button className="h-11 md:h-8" disabled={!canContinue} onClick={() => setStep((s) => s + 1)}>
              Continue<ArrowRight data-icon="inline-end" />
            </Button>
          </SheetFooter>
        )}
      </SheetContent>
    </Sheet>
  )
}
