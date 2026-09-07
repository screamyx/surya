// Rust: app/crates/ui/src/inbox/chrome.rs (state_color, dot, badge, heading, empty_state).
// The rail needs all five state tones; screens/chrome.tsx carries only the two
// the needs-you list uses, so these live beside the rail until they are merged.
import type { AgentGroup, AgentStateKind } from '../Agents';

/// The tone a state is drawn in. Mirrors state_color() and kind_color().
export type Tone = 'accent' | 'warning' | 'danger' | 'busy' | 'success' | 'faint';

export function stateTone(state: AgentStateKind): Tone {
  if (state === 'needs-you-permission') return 'accent';
  if (state === 'needs-you-question') return 'warning';
  if (state === 'needs-you-failed') return 'danger';
  if (state === 'stopped') return 'danger';
  if (state === 'working') return 'busy';
  if (state === 'done') return 'success';
  return 'faint';
}

/// A round status dot, for an agent row. Rust: dot(), size(px(6.0)).
export function dot(tone: Tone) {
  return <span aria-hidden="true" className={
    tone === 'accent' ? 'flex-none size-1.5 rounded-full bg-accent'
      : tone === 'warning' ? 'flex-none size-1.5 rounded-full bg-warning'
        : tone === 'danger' ? 'flex-none size-1.5 rounded-full bg-danger'
          : tone === 'busy' ? 'flex-none size-1.5 rounded-full bg-busy'
            : tone === 'success' ? 'flex-none size-1.5 rounded-full bg-success'
              : 'flex-none size-1.5 rounded-full bg-text-faint'} />;
}

/// The roll-up count a parent shows for a blocked descendant. Rust: badge().
export function badge(label: string, tone: Tone) {
  return <span className={
    tone === 'accent' ? 'flex-none px-1.5 py-0.25 rounded-full bg-accent/14 text-ui-10 font-medium text-accent'
      : tone === 'warning' ? 'flex-none px-1.5 py-0.25 rounded-full bg-warning/14 text-ui-10 font-medium text-warning'
        : tone === 'danger' ? 'flex-none px-1.5 py-0.25 rounded-full bg-danger/14 text-ui-10 font-medium text-danger'
          : tone === 'busy' ? 'flex-none px-1.5 py-0.25 rounded-full bg-busy/14 text-ui-10 font-medium text-busy'
            : tone === 'success' ? 'flex-none px-1.5 py-0.25 rounded-full bg-success/14 text-ui-10 font-medium text-success'
              : 'flex-none px-1.5 py-0.25 rounded-full bg-text-faint/14 text-ui-10 font-medium text-text-faint'}>{label}</span>;
}

/// The four group headings, in the order decision 15 gives.
/// Rust: model.rs AgentGroup::heading().
export function groupHeading(group: AgentGroup) {
  const text = group === 'waiting-for-you' ? 'Waiting for you'
    : group === 'running' ? 'Running'
      : group === 'done' ? 'Done' : 'Idle';
  return <div className="px-1 pt-2.5 pb-1 text-ui-10 font-medium text-text-faint">{text}</div>;
}

/// The quiet state. Rust: empty_state().
export function emptyState(text: string) {
  return <div className="flex flex-col items-center justify-center py-10 text-ui-13 text-text-faint">{text}</div>;
}
