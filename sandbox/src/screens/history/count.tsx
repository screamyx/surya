// Rust: app/crates/ui/src/history.rs (GitHistoryCount::render, line 454).
// Hosted by the Changes surface strip (changes.rs), not by GitHistory itself.
export type GitHistoryCountProps = { count: number | null; branch: string | null };
export function GitHistoryCount({ count, branch }: GitHistoryCountProps) {
  return (
    <div className="h-full min-w-0 flex items-center gap-2.5">
      {count !== null && (
        <div className="flex-none whitespace-nowrap text-ui-11 text-text-muted">
          {count === 1 ? '1 commit' : `${count} commits`}
        </div>
      )}
      {branch !== null && (
        <div className="min-w-0 truncate text-ui-12 text-text-muted">{branch}</div>
      )}
    </div>
  );
}
