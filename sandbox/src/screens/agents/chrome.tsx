// Rust: app/crates/ui/src/inbox/chrome.rs (state_color) and inbox/model.rs
// (AgentGroup::heading). The two mappings that need this screen's model types.
// The tone-keyed primitives they feed live in ../chrome.tsx, mirroring the same
// Rust file; nothing here re-implements them.
import type { AgentGroup, AgentStateKind } from '../Agents';
import type { Tone } from '../chrome';

/// chrome.rs::state_color. Which tone a rail row is drawn in.
export function stateTone(state: AgentStateKind): Tone {
  if (state === 'needs-you-permission') return 'accent';
  if (state === 'needs-you-question') return 'warning';
  if (state === 'needs-you-failed') return 'danger';
  if (state === 'stopped') return 'danger';
  if (state === 'working') return 'busy';
  if (state === 'done') return 'success';
  return 'faint';
}

/// The four group headings, in the order decision 15 gives.
/// Rust: model.rs AgentGroup::heading().
export function groupHeading(group: AgentGroup) {
  const text = group === 'waiting-for-you' ? 'Waiting for you'
    : group === 'running' ? 'Running'
      : group === 'done' ? 'Done' : 'Idle';
  return <div className="px-1 pt-2.5 pb-1 text-ui-10 font-medium text-text-faint">{text}</div>;
}
