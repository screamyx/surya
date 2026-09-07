// Rust: app/crates/ui/src/frost.rs - frosted() (27) and MENU_BLUR (24).
//
// The native element paints a 44px backdrop blur under a popover card, but only
// when Theme::is_frost() is true, and that is `cfg!(any(macos, linux))`. On
// Windows, the product, is_frost() is false and the element is a pass-through:
// the card paints its opaque surface_overlay fill and nothing blurs behind it.
// That pass-through is what this file carries, because backdrop blur is banned
// by the guide with no GPUI pair admitted. If the glass path ever needs a
// stand-in here, the admitted substitute is `bg-surface-overlay/88` in place of
// the opaque fill; it cannot reproduce the blur, only the translucency.
import type { ReactNode } from 'react';

export function frosted(children: ReactNode) {
  return children;
}
