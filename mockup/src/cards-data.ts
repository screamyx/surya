import type { CardSample } from "./data"

export const cards: CardSample[] = [
  { type: "vehicle", stockNo: "KSS-0412", title: "2021 Toyota Alphard 2.5 SC", price: "RM 268,800", days: 34, photo: "https://picsum.photos/seed/alphard/640/400", status: "In stock" },
  { type: "table", title: "Leads with no follow-up date", columns: ["Lead", "Car", "Came in", "Salesperson"], rows: [["Ahmad F.", "Alphard SC", "2 days ago", "Zul"], ["Mei Ling", "Vellfire ZG", "4 days ago", "Farah"], ["Rajesh K.", "Harrier Z", "6 days ago", "Zul"]] },
  { type: "form", title: "Set the follow-up", fields: [{ label: "Lead", type: "text", value: "Ahmad F." }, { label: "When", type: "date", value: "2026-09-06" }, { label: "Salesperson", type: "select", value: "Zul", options: ["Zul", "Farah", "Hafiz"] }, { label: "Note", type: "text", value: "Ask about trade-in" }], submit: "Save follow-up" },
  { type: "approval", title: "Write 3 leads", summary: "Set next_follow_up_at to tomorrow 08:00 for the three leads above.", code: "Lead::whereIn('id', [412, 418, 421])\n    ->update(['next_follow_up_at' => now()->addDay()->setTime(8, 0)]);" },
  { type: "diff-summary", title: "What changed", files: [{ path: "backend/app/Models/Lead.php", added: 12, removed: 0 }, { path: "backend/database/migrations/2026_09_05_000001_add_follow_up_to_leads.php", added: 28, removed: 0 }, { path: "backend/app/Jobs/SendFollowUpReminder.php", added: 24, removed: 0 }], note: "Two new columns, one job, no route changes." },
  { type: "metric", title: "Leads followed up this week", value: "38", delta: "+12 vs last week", series: [4, 6, 3, 8, 7, 5, 5] },
]

// Cards a workspace brings with it, read from its `.surya/cards` folder. Each one is a schema plus a layout over surya's
// primitives, so the mockup renders them through the same six shapes. The names below are the workspace's, not surya's.
export type WorkspaceCard = { name: string; description: string; shape: CardSample["type"]; file: string; sample: CardSample }
export const workspaceCards: Record<string, WorkspaceCard[]> = {
  "project-jag": [
    { name: "vehicle", description: "One car in stock: photo, price, days in stock, quick actions", shape: "vehicle", file: ".surya/cards/vehicle.json", sample: cards[0] },
    { name: "lead", description: "One lead with the next follow-up and who owns it", shape: "form", file: ".surya/cards/lead.json", sample: { type: "form", title: "Ahmad F. · Alphard SC", fields: [{ label: "Next follow-up", type: "date", value: "2026-09-06" }, { label: "Salesperson", type: "select", value: "Zul", options: ["Zul", "Farah", "Hafiz"] }, { label: "Note", type: "text", value: "Ask about trade-in" }], submit: "Save" } },
  ],
  "kss-marketing": [
    { name: "campaign", description: "Spend, reach and results for one Meta campaign, with a 7-day line", shape: "metric", file: ".surya/cards/campaign.json", sample: { type: "metric", title: "CTWA · Alphard reels · spend this week", value: "RM 1,240", delta: "38 conversations, RM 32.60 each", series: [120, 180, 160, 210, 190, 200, 180] } },
    { name: "reel", description: "One catalog reel with its publish state per placement", shape: "table", file: ".surya/cards/reel.json", sample: { type: "table", title: "Reel KSS-0412 · placements", columns: ["Placement", "State", "Views"], rows: [["Catalog", "Published", "1,204"], ["Reels", "Published", "8,911"], ["Stories", "Queued", "-"]] } },
  ],
}
