// Screen: tasks. Builder replaces this file. Read src/data.ts and src/components/app-shell.tsx first.
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty"

export function TasksScreen() {
  return (
    <Empty className="min-h-[60vh]">
      <EmptyHeader>
        <EmptyTitle>tasks</EmptyTitle>
        <EmptyDescription>Not built yet.</EmptyDescription>
      </EmptyHeader>
    </Empty>
  )
}
