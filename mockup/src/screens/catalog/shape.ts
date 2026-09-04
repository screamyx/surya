// data.ts names the record shape after the first thing it was ever sampled with.
// The catalog shows what the shape actually is.
import type { CardSample } from "@/data"

export const shapeName = (card: CardSample) => ("photo" in card ? "record" : card.type)
