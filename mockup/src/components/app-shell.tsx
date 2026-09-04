import { Link, NavLink, Outlet, useLocation, useParams } from "react-router"
import { Bell, FolderTree, Home, Inbox, LayoutGrid, ListTodo, Monitor, Plus, Settings, Sparkles } from "lucide-react"
import {
  Sidebar, SidebarContent, SidebarFooter, SidebarGroup, SidebarGroupContent, SidebarGroupLabel, SidebarHeader, SidebarInset,
  SidebarMenu, SidebarMenuBadge, SidebarMenuButton, SidebarMenuItem, SidebarMenuSub, SidebarMenuSubButton, SidebarMenuSubItem,
  SidebarProvider, SidebarRail, SidebarTrigger,
} from "@/components/ui/sidebar"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Separator } from "@/components/ui/separator"
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbPage, BreadcrumbSeparator } from "@/components/ui/breadcrumb"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { StatusDot } from "@/components/status"
import { inbox, workspaces } from "@/data"
import { cn } from "@/lib/utils"

const needsYou = inbox.filter((i) => i.kind === "permission" || i.kind === "question").length

function WorkspaceNav() {
  const { ws, id } = useParams()
  const { pathname } = useLocation()
  return (
    <>
      {workspaces.map((w) => (
        <SidebarGroup key={w.id}>
          <SidebarGroupLabel className="justify-between">
            <span className="truncate">{w.name}</span>
            <span className="text-muted-foreground font-normal">{w.branch}</span>
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {w.agents.map((a) => (
                <SidebarMenuItem key={a.id}>
                  <SidebarMenuButton render={<Link to={`/w/${w.id}/agent/${a.id}`} />} isActive={ws === w.id && id === a.id} tooltip={a.summary}>
                    <StatusDot status={a.status} />
                    <span className="truncate">{a.name}</span>
                    <span className="text-muted-foreground ml-auto truncate text-xs">{a.model.replace("claude-", "").replace("gpt-", "")}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
              <SidebarMenuItem>
                <SidebarMenuSub className="mx-0 border-0 px-0">
                  {[
                    { to: `/w/${w.id}/agents`, icon: Sparkles, label: "Agents" },
                    { to: `/w/${w.id}/tasks`, icon: ListTodo, label: "Tasks" },
                    { to: `/w/${w.id}/preview`, icon: Monitor, label: "Preview" },
                    { to: `/w/${w.id}/files`, icon: FolderTree, label: "Files" },
                  ].map((l) => (
                    <SidebarMenuSubItem key={l.to}>
                      <SidebarMenuSubButton render={<Link to={l.to} />} isActive={pathname === l.to} size="sm">
                        <l.icon />
                        <span>{l.label}</span>
                      </SidebarMenuSubButton>
                    </SidebarMenuSubItem>
                  ))}
                </SidebarMenuSub>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      ))}
    </>
  )
}

function Crumbs() {
  const { ws, id } = useParams()
  const { pathname } = useLocation()
  const leaf = pathname.split("/").filter(Boolean).pop() ?? "home"
  const leafLabel: Record<string, string> = { inbox: "Needs you", catalog: "Cards", settings: "Settings", new: "New ask", agents: "Agents", tasks: "Tasks", preview: "Preview", files: "Files" }
  return (
    <Breadcrumb>
      <BreadcrumbList>
        <BreadcrumbItem className="hidden md:block"><BreadcrumbLink render={<Link to="/" />}>surya</BreadcrumbLink></BreadcrumbItem>
        {ws && (<><BreadcrumbSeparator className="hidden md:block" /><BreadcrumbItem><BreadcrumbLink render={<Link to={`/w/${ws}/agents`} />}>{ws}</BreadcrumbLink></BreadcrumbItem></>)}
        {(id || leafLabel[leaf]) && (<><BreadcrumbSeparator className={ws ? undefined : "hidden md:block"} /><BreadcrumbItem><BreadcrumbPage>{id ?? leafLabel[leaf]}</BreadcrumbPage></BreadcrumbItem></>)}
      </BreadcrumbList>
    </Breadcrumb>
  )
}

function MobileNav() {
  const { ws } = useParams()
  const w = ws ?? workspaces[0].id
  const items = [
    { to: "/", icon: Home, label: "Home", end: true },
    { to: "/inbox", icon: Inbox, label: "Needs you", badge: needsYou },
    { to: `/w/${w}/agents`, icon: Sparkles, label: "Agents" },
    { to: `/w/${w}/preview`, icon: Monitor, label: "Preview" },
    { to: `/w/${w}/tasks`, icon: ListTodo, label: "Tasks" },
  ]
  return (
    <nav className="bg-background/95 supports-[backdrop-filter]:bg-background/80 fixed inset-x-0 bottom-0 z-40 grid grid-cols-5 border-t backdrop-blur md:hidden" style={{ paddingBottom: "env(safe-area-inset-bottom)" }}>
      {items.map((i) => (
        <NavLink key={i.to} to={i.to} end={i.end} className={({ isActive }) => cn("text-muted-foreground relative flex flex-col items-center gap-0.5 py-2 text-[11px]", isActive && "text-foreground")}>
          <i.icon className="size-5" />
          {i.label}
          {i.badge ? <Badge variant="destructive" className="absolute top-1 right-1/2 -mr-5 h-4 min-w-4 px-1 text-[10px]">{i.badge}</Badge> : null}
        </NavLink>
      ))}
    </nav>
  )
}

export function AppShell() {
  return (
    <SidebarProvider>
      <Sidebar collapsible="icon">
        <SidebarHeader>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton size="lg" render={<Link to="/" />}>
                <div className="bg-primary text-primary-foreground flex aspect-square size-8 items-center justify-center rounded-lg"><Sparkles className="size-4" /></div>
                <div className="grid flex-1 text-left leading-tight"><span className="truncate font-semibold">surya</span><span className="text-muted-foreground truncate text-xs">pc-ajim, 3 workspaces</span></div>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarHeader>
        <SidebarContent>
          <SidebarGroup>
            <SidebarGroupContent>
              <SidebarMenu>
                <SidebarMenuItem><SidebarMenuButton render={<Link to="/new" />} tooltip="New ask"><Plus /><span>New ask</span></SidebarMenuButton></SidebarMenuItem>
                <SidebarMenuItem>
                  <SidebarMenuButton render={<Link to="/inbox" />} tooltip="Needs you"><Inbox /><span>Needs you</span></SidebarMenuButton>
                  {needsYou > 0 && <SidebarMenuBadge className="bg-destructive text-destructive-foreground rounded-full">{needsYou}</SidebarMenuBadge>}
                </SidebarMenuItem>
                <SidebarMenuItem><SidebarMenuButton render={<Link to="/catalog" />} tooltip="Cards"><LayoutGrid /><span>Cards</span></SidebarMenuButton></SidebarMenuItem>
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
          <WorkspaceNav />
        </SidebarContent>
        <SidebarFooter>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton render={<Link to="/settings" />} tooltip="Settings"><Settings /><span>Settings</span></SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarFooter>
        <SidebarRail />
      </Sidebar>
      <SidebarInset className="min-w-0">
        <header className="bg-background/95 sticky top-0 z-30 flex h-12 shrink-0 items-center gap-2 border-b px-3 backdrop-blur">
          <SidebarTrigger className="-ml-1" />
          <Separator orientation="vertical" className="mr-1 h-4" />
          <Crumbs />
          <div className="ml-auto flex items-center gap-1">
            <Button variant="ghost" size="icon" nativeButton={false} render={<Link to="/inbox" />} aria-label="Needs you" className="relative">
              <Bell />
              {needsYou > 0 && <span className="bg-destructive absolute top-1.5 right-1.5 size-2 rounded-full" />}
            </Button>
            <Button size="sm" nativeButton={false} render={<Link to="/new" />} className="hidden md:inline-flex"><Plus data-icon="inline-start" />New ask</Button>
            <Avatar className="size-7"><AvatarFallback>AZ</AvatarFallback></Avatar>
          </div>
        </header>
        <main className="min-w-0 flex-1 pb-20 md:pb-0">
          <Outlet />
        </main>
        <MobileNav />
      </SidebarInset>
    </SidebarProvider>
  )
}
