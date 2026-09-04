import { Link, NavLink, Outlet, useLocation, useParams } from "react-router"
import { Bell, ChevronRight, Home, Inbox, LayoutGrid, ListTodo, Mail, Monitor, Plus, Server as ServerIcon, Settings, Sparkles } from "lucide-react"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import {
  Sidebar, SidebarContent, SidebarFooter, SidebarGroup, SidebarGroupContent, SidebarGroupLabel, SidebarHeader, SidebarInset,
  SidebarMenu, SidebarMenuAction, SidebarMenuBadge, SidebarMenuButton, SidebarMenuItem, SidebarMenuSub, SidebarMenuSubButton, SidebarMenuSubItem,
  SidebarProvider, SidebarRail, SidebarTrigger,
} from "@/components/ui/sidebar"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Separator } from "@/components/ui/separator"
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbPage, BreadcrumbSeparator } from "@/components/ui/breadcrumb"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { StatusDot } from "@/components/status"
import { byPriority, inbox, needsYouCount, rollup, servers, settings, unreadFor, workspaces, type Agent, type Server } from "@/data"
import { cn } from "@/lib/utils"

const needsYou = inbox.filter((i) => i.kind === "permission" || i.kind === "question" || i.kind === "failed").length

// One row per agent. Spawned agents nest under their spawner, folded by default unless one of them needs you.
function AgentRow({ a, ws }: { a: Agent; ws: string }) {
  const { id } = useParams()
  const kids = byPriority(a.children)
  const hot = needsYouCount(a.children) > 0
  const row = (
    <SidebarMenuButton render={<Link to={`/w/${ws}/agent/${a.id}`} />} isActive={id === a.id} tooltip={a.summary} className="group/agent">
      <StatusDot status={rollup(a)} />
      <span className="truncate">{a.name}</span>
      <span className="text-muted-foreground min-w-0 flex-1 truncate text-xs">{a.summary}</span>
      {unreadFor(a.id) > 0 && <span className="text-muted-foreground flex shrink-0 items-center gap-0.5 text-[10px]"><Mail className="size-3" />{unreadFor(a.id)}</span>}
    </SidebarMenuButton>
  )
  if (!kids.length) return <SidebarMenuItem>{row}</SidebarMenuItem>
  return (
    <Collapsible defaultOpen={hot || id === a.id || kids.some((k) => k.id === id)} className="group/kids" render={<SidebarMenuItem />}>
      {row}
      <CollapsibleTrigger render={<SidebarMenuAction className="data-[panel-open]:rotate-90" aria-label="Show spawned agents" />}>
        <ChevronRight />
      </CollapsibleTrigger>
      <CollapsibleContent>
        <SidebarMenuSub>
          {kids.map((k) => (
            <SidebarMenuSubItem key={k.id}>
              <SidebarMenuSubButton render={<Link to={`/w/${ws}/agent/${k.id}`} />} isActive={id === k.id} size="sm">
                <StatusDot status={rollup(k)} />
                <span className="truncate">{k.name}</span>
                <span className="text-muted-foreground min-w-0 flex-1 truncate text-xs">{k.summary}</span>
              </SidebarMenuSubButton>
            </SidebarMenuSubItem>
          ))}
        </SidebarMenuSub>
      </CollapsibleContent>
    </Collapsible>
  )
}

// Servers are the top level, then workspaces, then agents. The server level hides when there is only one server.
function WorkspaceRow({ w, current }: { w: (typeof workspaces)[number]; current: string }) {
  const hot = needsYouCount(w.agents)
  const worst = w.agents.length ? rollup({ ...w.agents[0], status: "idle", children: w.agents }) : "idle"
  return (
    <Collapsible defaultOpen={w.id === current} render={<SidebarMenuItem />} className="group/ws">
      <SidebarMenuButton render={<Link to={`/w/${w.id}/agents`} />} isActive={w.id === current} tooltip={`${w.name} · ${w.branch}`} className="font-medium">
        <span className="bg-muted text-foreground relative grid size-5 shrink-0 place-items-center rounded-md text-[10px] font-semibold uppercase">
          {w.name.slice(0, 2)}
          <StatusDot status={worst} className="absolute -top-0.5 -right-0.5 size-2 ring-2 ring-sidebar" />
        </span>
        <span className="truncate">{w.name}</span>
      </SidebarMenuButton>
      {hot > 0 && <SidebarMenuBadge className="bg-destructive text-destructive-foreground right-12 rounded-full">{hot}</SidebarMenuBadge>}
      <Tooltip>
        <TooltipTrigger render={<SidebarMenuAction className="right-6 opacity-0 group-hover/ws:opacity-100 max-md:opacity-100" render={<Link to={`/new?ws=${w.id}`} />} aria-label={`New agent in ${w.name}`} />}>
          <Plus />
        </TooltipTrigger>
        <TooltipContent side="right">New agent in {w.name}</TooltipContent>
      </Tooltip>
      <CollapsibleTrigger render={<SidebarMenuAction className="data-[panel-open]:rotate-90" aria-label="Toggle workspace" />}>
        <ChevronRight />
      </CollapsibleTrigger>
      <CollapsibleContent>
        <SidebarMenu className="mt-0.5 ml-1 gap-0.5">
          {byPriority(w.agents).map((a) => <AgentRow key={a.id} a={a} ws={w.id} />)}
        </SidebarMenu>
      </CollapsibleContent>
    </Collapsible>
  )
}

function ServerGroup({ srv, current }: { srv: Server; current: string }) {
  const list = workspaces.filter((w) => w.serverId === srv.id)
  const hot = list.reduce((n, w) => n + needsYouCount(w.agents), 0)
  const mine = list.some((w) => w.id === current)
  const off = srv.state !== "online"
  return (
    <Collapsible defaultOpen={mine || (srv.home && !off)} render={<SidebarGroup className="py-1" />}>
      <CollapsibleTrigger nativeButton={false} render={<SidebarGroupLabel className="hover:bg-sidebar-accent w-full cursor-pointer justify-start gap-2 rounded-md" />}>
        <ChevronRight className="size-3 transition-transform data-[panel-open]:rotate-90" />
        <ServerIcon className="size-3.5" />
        <span className={cn("truncate", off && "text-muted-foreground")}>{srv.name}</span>
        {srv.home && <span className="text-muted-foreground text-[10px] font-normal">home</span>}
        <span className="ml-auto flex items-center gap-1.5">
          {off ? <span className="text-muted-foreground text-[10px] font-normal">offline</span> : <span className="text-muted-foreground text-[10px] font-normal">{list.length} ws</span>}
          {hot > 0 && <span className="bg-destructive text-destructive-foreground grid size-4 place-items-center rounded-full text-[10px]">{hot}</span>}
          <span className={cn("size-1.5 rounded-full", off ? "bg-border" : "bg-primary")} />
        </span>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <SidebarGroupContent>
          <SidebarMenu>
            {list.map((w) => <WorkspaceRow key={w.id} w={w} current={current} />)}
            {list.length === 0 && <div className="text-muted-foreground px-2 py-1.5 text-xs">{off ? "Not reachable. Check it in Settings." : "No workspaces yet."}</div>}
          </SidebarMenu>
        </SidebarGroupContent>
      </CollapsibleContent>
    </Collapsible>
  )
}

function WorkspaceNav() {
  const { ws } = useParams()
  const current = ws ?? workspaces[0].id
  if (servers.length === 1) {
    return (
      <SidebarGroup>
        <SidebarGroupLabel>Workspaces</SidebarGroupLabel>
        <SidebarGroupContent><SidebarMenu>{workspaces.map((w) => <WorkspaceRow key={w.id} w={w} current={current} />)}</SidebarMenu></SidebarGroupContent>
      </SidebarGroup>
    )
  }
  return <>{servers.map((srv) => <ServerGroup key={srv.id} srv={srv} current={current} />)}</>
}

function Crumbs() {
  const { ws, id } = useParams()
  const { pathname } = useLocation()
  const leaf = pathname.split("/").filter(Boolean).pop() ?? "home"
  const leafLabel: Record<string, string> = { inbox: "Needs you", catalog: "Cards", settings: "Settings", new: "New ask", agents: "Agents", tasks: "Tasks", preview: "Preview", files: "Files", messages: "Messages" }
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
                <div className="grid flex-1 text-left leading-tight"><span className="truncate font-semibold">surya</span><span className="text-muted-foreground truncate text-xs">{servers.length} servers · {workspaces.length} workspaces</span></div>
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
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
          <WorkspaceNav />
        </SidebarContent>
        <SidebarFooter>
          <SidebarMenu>
            <SidebarMenuItem><SidebarMenuButton render={<Link to="/catalog" />} tooltip="Cards"><LayoutGrid /><span>Cards</span></SidebarMenuButton></SidebarMenuItem>
            <SidebarMenuItem><SidebarMenuButton render={<Link to="/settings" />} tooltip="Settings"><Settings /><span>Settings</span></SidebarMenuButton></SidebarMenuItem>
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
            <Avatar className="size-7"><AvatarFallback>{settings.user.name.split(" ").map((p) => p[0]).join("").slice(0, 2).toUpperCase()}</AvatarFallback></Avatar>
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
