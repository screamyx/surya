// Rust: app/crates/ui/src/pickers.rs - render_checkout_popover (2928), the
// checkout-kind dropdown: two rows, "Current checkout" or "Current worktree"
// for the local pick, and "New worktree".
import { icon, menuRowNav } from './popover';
import type { CheckoutKind } from '../../fixtures/pickers';

export type CheckoutPopoverProps = {
  current: CheckoutKind; hasWorktree: boolean; active: number;
  onPick: (kind: CheckoutKind) => void; onHover: (index: number) => void;
};

export function renderCheckoutPopover(props: CheckoutPopoverProps) {
  const localLabel = props.hasWorktree ? 'Current worktree' : 'Current checkout';
  return (
    <div className="flex flex-col gap-0.5">
      {menuRowNav('checkout-row-0', props.current === 'local', props.active === 0,
        () => props.onPick('local'), () => props.onHover(0),
        <>
          <span className="flex-none text-text-muted">
            {props.hasWorktree ? icon('folder-with-files', 14) : icon('folder', 14)}
          </span>
          <span className="flex-1 min-w-0 truncate">{localLabel}</span>
        </>)}
      {menuRowNav('checkout-row-1', props.current === 'new-worktree', props.active === 1,
        () => props.onPick('new-worktree'), () => props.onHover(1),
        <>
          <span className="flex-none text-text-muted">{icon('folder-with-files', 14)}</span>
          <span className="flex-1 min-w-0 truncate">New worktree</span>
        </>)}
    </div>
  );
}
