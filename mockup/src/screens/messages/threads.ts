// Thread model for the Messages tab. Mail is addressed to an agent id or to a channel
// ("#project-jag"), so a thread is either the workspace channel or one agent.
import { flatten, messages, unreadFor, type Agent, type Message, type Workspace } from "@/data"

export type Thread =
  | { kind: "channel"; id: string; label: string; unread: number; last?: Message }
  | { kind: "direct"; id: string; label: string; agent: Agent; unread: number; last?: Message }

export const isChannel = (address: string) => address.startsWith("#")
const involves = (m: Message, id: string) => m.from === id || m.to === id
const newest = (rows: Message[]) => [...rows].sort((a, b) => a.at.localeCompare(b.at)).pop()

export const byTime = (rows: Message[]) => [...rows].sort((a, b) => a.at.localeCompare(b.at))

// Everything posted to the workspace channel, oldest first.
export const channelMail = (channel: string) => byTime(messages.filter((m) => m.to === channel))

// You and one agent, both directions.
export const directMail = (id: string) =>
  byTime(messages.filter((m) => (m.from === "you" && m.to === id) || (m.from === id && m.to === "you")))

// Mail between this agent and other agents, grouped by the other agent.
export const crossMail = (id: string): { other: string; rows: Message[] }[] => {
  const rows = messages.filter((m) => involves(m, id) && m.from !== "you" && m.to !== "you" && !isChannel(m.to))
  const groups = new Map<string, Message[]>()
  for (const m of byTime(rows)) {
    const other = m.from === id ? m.to : m.from
    groups.set(other, [...(groups.get(other) ?? []), m])
  }
  return [...groups].map(([other, rows]) => ({ other, rows }))
}

// The list: the channel first, then every agent in the workspace, unread first.
export function threadsFor(w: Workspace): Thread[] {
  const channel = `#${w.id}`
  const head: Thread = {
    kind: "channel",
    id: channel,
    label: channel,
    unread: unreadFor(channel),
    last: newest(channelMail(channel)),
  }
  const rest: Thread[] = flatten(w.agents).map((a) => ({
    kind: "direct" as const,
    id: a.id,
    label: a.id,
    agent: a,
    unread: unreadFor(a.id),
    last: newest(messages.filter((m) => involves(m, a.id))),
  }))
  rest.sort((x, y) => y.unread - x.unread || (y.last?.at ?? "").localeCompare(x.last?.at ?? ""))
  return [head, ...rest]
}

// A stable delivery id for a message you send in the mockup. Same address, same id.
export const deliveryFor = (address: string) =>
  "d-" + [...address].reduce((h, c) => (h * 33 + c.charCodeAt(0)) % 65536, 7).toString(16).padStart(4, "0")
