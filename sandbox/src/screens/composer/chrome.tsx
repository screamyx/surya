// Rust: app/crates/ui/src/badges.rs (render).
// The popover and button helpers this screen used to carry are now in
// ../popover.tsx, mirroring popover.rs, shared with the spaces sidebar.
import type { IconName } from '../../icons';
import { renderIcon } from '../../icons';

/// badges.rs::render - the 24px pill the comments chip wears.
export function messageBadge(icon: IconName, label: string) {
  return (
    <span className="h-6 flex flex-row items-center gap-1.5 px-2 rounded-lg bg-wash/5 text-ui-12 font-medium text-text-muted">
      <span className="size-3 flex-none text-text-muted/88">{renderIcon(icon)}</span>
      {label}
    </span>
  );
}
