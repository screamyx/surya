// Stop is a control interrupt, not a kill. The running command dies in about 2 seconds, the session
// stays resumable. Probed 2026-09-05, docs/probes/headless-controls-2026-09-05.md.
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription,
  AlertDialogFooter, AlertDialogHeader, AlertDialogMedia, AlertDialogTitle } from "@/components/ui/alert-dialog"
import { OctagonX } from "lucide-react"

export function StopDialog({ open, onOpenChange, name, onConfirm }: {
  open: boolean; onOpenChange: (v: boolean) => void; name: string; onConfirm: () => void
}) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogMedia className="text-destructive"><OctagonX /></AlertDialogMedia>
          <AlertDialogTitle>Stop {name}?</AlertDialogTitle>
          <AlertDialogDescription>
            The current command is killed within 2 seconds. Background tasks {name} started keep running.
            You can resume the session later.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction variant="destructive" onClick={onConfirm}>Stop</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
