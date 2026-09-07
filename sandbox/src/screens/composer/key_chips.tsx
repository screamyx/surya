// Rust: app/crates/ui/src/key_chips.rs (summon_label, submit_label, jump_label)
// over settings.rs::badge_combo_on. Windows is the product, so the default is
// the WORD "Ctrl", not the Mac Command glyph.
export function summonLabel(mac: boolean) { return mac ? '⌘K' : 'Ctrl+K'; }
export function submitLabel(mac: boolean) { return mac ? '⌘Enter' : 'Ctrl+Enter'; }
export function jumpLabel(mac: boolean, slot: number) {
  return mac ? `⌘${slot}` : `Ctrl+${slot}`;
}
