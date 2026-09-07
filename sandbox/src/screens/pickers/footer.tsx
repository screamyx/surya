// Rust: app/crates/ui/src/pickers.rs - render_footer (2454). The composer
// footer row: checkout-kind on the left edge and ref on the right, only when
// the picked project has git. A locked session swaps both chips for read-only
// labels; the target moved to the titlebar.
import type { ReactNode } from 'react';
import { footerChip, footerLabel, overlayAboveFooter } from './chips';
import type { CheckoutKind } from '../../fixtures/pickers';

export type FooterProps = {
  git: boolean; locked: boolean; checkout: CheckoutKind; hasWorktree: boolean;
  branch: string | null; openKind: string | null;
  onToggleCheckout: () => void; onToggleBranch: () => void;
  checkoutPopover: ReactNode; branchPopover: ReactNode;
};

/// ref_label: "From <ref>" only when a NEW worktree will be created off it.
function refLabel(checkout: CheckoutKind, branch: string | null) {
  if (!branch) return 'Select ref';
  return checkout === 'new-worktree' ? `From ${branch}` : branch;
}

function checkoutLabel(checkout: CheckoutKind, hasWorktree: boolean) {
  if (checkout === 'new-worktree') return 'New worktree';
  return hasWorktree ? 'Current worktree' : 'Current checkout';
}

export function renderFooter(props: FooterProps) {
  if (!props.git) return null;
  if (props.locked) {
    return (
      <div className="w-full flex flex-row items-center justify-between gap-2 px-2.5">
        <div className="flex flex-row items-center min-w-0">
          {footerLabel(props.hasWorktree ? 'folder-with-files' : 'folder',
            props.hasWorktree ? 'Worktree' : 'Local checkout')}
        </div>
        <div className="flex flex-row items-center gap-1 min-w-0">
          {footerLabel('git-branch', props.branch ?? 'No ref')}
        </div>
      </div>
    );
  }
  const kindIcon = props.checkout === 'local' && !props.hasWorktree ? 'folder' : 'folder-with-files';
  return (
    <div className="w-full flex flex-row items-center justify-between gap-2 px-2.5">
      <div className="relative flex flex-row items-center min-w-0">
        {footerChip('picker-checkout', kindIcon, checkoutLabel(props.checkout, props.hasWorktree),
          props.openKind === 'checkout', false, props.onToggleCheckout)}
        {props.openKind === 'checkout' && overlayAboveFooter('start', props.checkoutPopover)}
      </div>
      <div className="relative flex flex-row items-center min-w-0">
        {footerChip('picker-branch', 'git-branch', refLabel(props.checkout, props.branch),
          props.openKind === 'branch', false, props.onToggleBranch)}
        {props.openKind === 'branch' && overlayAboveFooter('end', props.branchPopover)}
      </div>
    </div>
  );
}
