// Rust: app/crates/ui/src/tasks/chips.rs (count_chip, owner_chip, link_chip,
// owner_label, short_id, link_label, preview). Presentational only, no state.
// ink(0.08) and ink(0.03) map to the nearest admitted wash alphas (10 and 5);
// hairline(0.10) maps to border-wash/10. font_mono on link_chip has no pair.
export function countChip(count: number) {
  return <span className="px-2 py-0.25 rounded-full bg-wash/10 text-ui-11 text-text-muted">{count}</span>;
}

/// "you" for the user (or nobody yet), else the agent id with a dot.
export function ownerLabel(owner: string | undefined): [string, boolean] {
  const value = (owner ?? '').trim();
  if (value === '' || value === 'user' || value === 'you') return ['you', false];
  return [value, true];
}

export function ownerChip(owner: string | undefined) {
  const [label, isAgent] = ownerLabel(owner);
  return (
    <span className="flex flex-row items-center gap-1.5 px-2 py-0.5 rounded-full border border-wash/10 bg-wash/5 text-ui-11 text-text-muted">
      {isAgent && <span className="size-1.5 flex-none rounded-full bg-accent" />}
      {label}
    </span>
  );
}

export function linkChip(link: string) {
  return (
    <span className="flex flex-row items-center gap-1.5 px-2 py-0.5 rounded-full border border-wash/10 bg-wash/5 text-ui-11 text-text-muted">
      {linkLabel(link)}
    </span>
  );
}

/// `t-1` stays; a UUID shows its first 6 characters.
export function shortId(id: string) {
  return id.length <= 8 ? id : [...id].slice(0, 6).join('');
}

/// `https://github.com/o/r/pull/12` becomes `o/r#12`; other URLs show the host.
export function linkLabel(link: string) {
  const stripped = link.trim().replace(/^https:\/\//, '').replace(/^http:\/\//, '');
  const parts = stripped.split('/').filter(p => p !== '');
  if (parts.length === 0) return link;
  if (parts.length >= 5 && parts[0] === 'github.com' && (parts[3] === 'pull' || parts[3] === 'issues')) {
    return `${parts[1]}/${parts[2]}#${parts[4]}`;
  }
  return parts[0];
}

/// First `max` characters of the first line, with an ellipsis when cut.
export function preview(text: string, max: number) {
  const line = text.split('\n')[0] ?? '';
  const chars = [...line];
  if (chars.length <= max) return line;
  return `${chars.slice(0, max).join('').trimEnd()}…`;
}
