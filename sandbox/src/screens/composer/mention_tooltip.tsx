// Rust: app/crates/ui/src/composer.rs, MentionPathTooltip (2854) and its Render
// impl (2861). Native mounts it with window.set_tooltip, a window-level layer;
// here the Composer holds it above the pill, because the pill clips its own
// overflow and no admitted class escapes that.
export function MentionPathTooltip({ path }: { path: string }) {
  return (
    <div role="tooltip" data-tooltip="file-mention-path"
      className="mr-auto h-6 max-w-96 flex items-center px-2 rounded-sm border border-border-strong bg-surface-raised text-ui-11 text-text-muted whitespace-nowrap overflow-hidden">
      {path}
    </div>
  );
}
