// The Browser engine's own state. Decision 21: the second preview engine is a real Chrome
// owned by the daemon, painted into the pane as a frame stream, and "the profile persists on
// the daemon, so the user signs in once, from any device, phone included".
//
// None of this belongs in src/data.ts: it describes the daemon's browser, not the workspace.
// Every site here is invented and sits on the reserved .example TLD (RFC 2606), so nothing
// in the mockup borrows a real brand, logo or product name.

import type { Pin } from "@/data"

export type Engine = "app" | "browser"

// How the daemon reached the page. "daemon" is the persistent Chrome it owns and is always
// the default; "desktop" is open-claude-in-chrome, which needs the user's machine awake.
export type Connection = "daemon" | "desktop"

export type SiteId = "ads" | "docs"

export type BrowserSite = {
  id: SiteId
  url: string
  host: string
  title: string
  // What the profile is signed in as on this site. The whole point of the daemon profile.
  account: string
  // Shown under the address in the menu, so the history reads as pages, not as strings.
  visited: string
  signIn: "account" | "public"
}

export const sites: BrowserSite[] = [
  {
    id: "ads",
    url: "https://ads.anchorpoint.example/campaigns/kss-auto",
    host: "ads.anchorpoint.example",
    title: "Anchorpoint Ads - KSS Auto",
    account: "zul@kssauto.example",
    visited: "12 minutes ago",
    signIn: "account",
  },
  {
    id: "docs",
    url: "https://docs.stockfeed.example/reference/vehicles",
    host: "docs.stockfeed.example",
    title: "Stockfeed docs - Vehicle feed",
    account: "Public page, no sign-in",
    visited: "yesterday",
    signIn: "public",
  },
]

export const siteById = (id: SiteId) => sites.find((s) => s.id === id) ?? sites[0]

// The daemon's Chrome profile. One profile, every device.
export const profile = {
  name: "Workshop profile",
  detail: "Signed in on the daemon since 14 Aug",
}

export const connections: { value: Connection; label: string; detail: string }[] = [
  {
    value: "daemon",
    label: "Use the daemon's Chrome",
    detail: "Runs on the server. Stays signed in. Paints on any device, phone included.",
  },
  {
    value: "desktop",
    label: "Use my desktop Chrome",
    detail: "For sites only your own profile can reach. Needs that desktop awake, and it cannot paint on a phone.",
  },
]

// The daemon's Chrome has no desktop to be unreachable from, so only the desktop option
// can fail, and only where it cannot paint.
export function paints(connection: Connection, compact: boolean) {
  return connection === "daemon" || !compact
}

// ---------------------------------------------------------------------------
// Page content for the two invented sites.

export type Campaign = {
  name: string
  status: "Active" | "Paused" | "In review"
  spend: string
  leads: number
  cpl: string
}

export const campaigns: Campaign[] = [
  { name: "Alphard SC - Klang Valley", status: "Active", spend: "RM 4,820", leads: 61, cpl: "RM 79" },
  { name: "Harrier Luxury - retarget", status: "Active", spend: "RM 2,140", leads: 33, cpl: "RM 65" },
  { name: "Trade-in valuation form", status: "In review", spend: "RM 1,905", leads: 24, cpl: "RM 79" },
  { name: "Vellfire ZG - lookalike 2%", status: "Paused", spend: "RM 1,260", leads: 9, cpl: "RM 140" },
  { name: "Perodua Alza - weekend push", status: "Active", spend: "RM 880", leads: 18, cpl: "RM 49" },
]

// `key` doubles as the class the daemon reports in a selector, so a pin on a tile reads
// like a selector a person could paste into the console.
export const adsTotals = [
  { key: "spend", label: "Spend", value: "RM 11,005", note: "last 7 days" },
  { key: "leads", label: "Leads", value: "145", note: "+22 vs prior week" },
  { key: "cost-per-lead", label: "Cost per lead", value: "RM 76", note: "-RM 8 vs prior week" },
]

export type DocsField = { name: string; type: string; note: string }

export const docsFields: DocsField[] = [
  { name: "stock_no", type: "string", note: "Your own reference. Must be unique per dealer." },
  { name: "title", type: "string", note: "Year, make, model, variant. 120 characters." },
  { name: "price_myr", type: "integer", note: "Asking price in ringgit. No decimals, no separators." },
  { name: "photos[]", type: "url[]", note: "Up to 24. The first one is the cover." },
  { name: "listed_at", type: "date-time", note: "ISO 8601. Sets the age badge on the listing." },
]

export const docsNav = [
  { label: "Getting started", active: false },
  { label: "Authentication", active: false },
  { label: "Vehicle feed", active: true },
  { label: "Lead webhooks", active: false },
  { label: "Rate limits", active: false },
  { label: "Errors", active: false },
]

// ---------------------------------------------------------------------------
// Pins the daemon already resolved on the browser engine. Decision 21: "Pins still
// resolve to a selector: the daemon asks Chrome which element is under the click, and
// stores selector plus crop as before." Same Pin shape as the app engine, so the bubble,
// the numbering and the list are one component for both.
export const browserPins: Record<SiteId, Pin[]> = {
  ads: [
    {
      id: "bp-1",
      workspaceId: "project-jag",
      x: 76,
      y: 33,
      selector: ".totals .cost-per-lead",
      note: "Read this number every morning and post it to the team thread.",
      by: "you",
      status: "open",
    },
    {
      id: "bp-2",
      workspaceId: "project-jag",
      x: 43,
      y: 64,
      selector: '.campaign-row[data-name="Trade-in valuation form"] .status',
      note: "Still in review after four days. Chase it or pause the spend.",
      by: "agent",
      status: "taken",
    },
  ],
  docs: [],
}
