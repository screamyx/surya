// Rust: app/crates/ui/src/shell.rs Shell::render_chat_row (line 4932), its
// status corner, and app/crates/ui/src/change_requests.rs pull_request_badge
// on the Sidebar surface. Render helpers on the same pane, not components.
import type { ChatRow, ChatStatus, SendState } from '../../fixtures/spaces';
import { renderIcon } from '../../icons';

/// spaces.rs status_dot_color, with render_chat_row's send-truth overrides in
/// front of it: an undelivered send is danger, a queued one warning. Native
/// alphas are 0.55 busy, 0.6 accent, 0.65 danger, 0.9 success, ink(0.14) idle.
export function statusTone(status: ChatStatus, send: SendState) {
  if (send === 'undelivered') return 'danger';
  if (send === 'queued') return 'warning';
  if (status === 'working') return 'busy';
  if (status === 'awaiting-input') return 'accent';
  if (status === 'errored') return 'danger';
  if (status === 'completed') return 'success';
  return 'idle';
}

/// render_chat_row's status_label. Idle rows have no word and show the
/// relative time in the corner instead.
export function statusLabel(status: ChatStatus, send: SendState): string | null {
  if (send === 'undelivered') return 'Failed';
  if (send === 'queued') return 'Queued';
  if (status === 'working') return 'Working';
  if (status === 'awaiting-input') return 'Input';
  if (status === 'errored') return 'Failed';
  if (status === 'completed') return 'Done';
  return null;
}

/// The status word, tinted by status_dot_color's tone and native alpha.
function statusWord(tone: string, label: string) {
  if (tone === 'danger') return <span className="text-ui-10 font-medium text-danger">{label}</span>;
  if (tone === 'warning') return <span className="text-ui-10 font-medium text-warning">{label}</span>;
  if (tone === 'busy') return <span className="text-ui-10 font-medium text-busy/50">{label}</span>;
  if (tone === 'accent') return <span className="text-ui-10 font-medium text-accent/50">{label}</span>;
  if (tone === 'success') return <span className="text-ui-10 font-medium text-success/88">{label}</span>;
  return <span className="text-ui-10 font-medium text-text-muted/50">{label}</span>;
}

/// The compact dot every non-Done, non-Working status wears.
function statusDot(tone: string) {
  if (tone === 'danger') return <span className="size-1.5 flex-none rounded-full bg-danger" />;
  if (tone === 'warning') return <span className="size-1.5 flex-none rounded-full bg-warning" />;
  if (tone === 'busy') return <span className="size-1.5 flex-none rounded-full bg-busy/50" />;
  if (tone === 'accent') return <span className="size-1.5 flex-none rounded-full bg-accent/50" />;
  if (tone === 'success') return <span className="size-1.5 flex-none rounded-full bg-success/88" />;
  return <span className="size-1.5 flex-none rounded-full bg-wash/14" />;
}

/// harness_brand_icon: the provider mark keeps its immutable brand color;
/// the unbranded marks inherit the row's subline tone.
function harnessMark(harness: ChatRow['harness']) {
  if (harness === 'claude') {
    return <span className="size-3.5 flex-none text-claude-brand">{renderIcon('claude-mark')}</span>;
  }
  if (harness === 'openai') {
    return <span className="size-3.5 flex-none text-text-muted/50">{renderIcon('openai-mark')}</span>;
  }
  return null;
}

/// pull_request_badge, Sidebar surface: 16px pinned, px 4, radius 4,
/// borderless 0.08 fill under 0.85 text of one tone, number in the mono face.
export function pullRequestBadge(number: string, state: 'open' | 'merged' | 'closed') {
  const label = '#' + number;
  if (state === 'merged') {
    return <span className="h-4 flex-none flex flex-row items-center px-1 rounded-sm bg-code-text/10 text-ui-10 font-medium text-code-text/88">{label}</span>;
  }
  if (state === 'closed') {
    return <span className="h-4 flex-none flex flex-row items-center px-1 rounded-sm bg-danger/10 text-ui-10 font-medium text-danger/88">{label}</span>;
  }
  return <span className="h-4 flex-none flex flex-row items-center px-1 rounded-sm bg-success/10 text-ui-10 font-medium text-success/88">{label}</span>;
}

/// The corner: the jump chip wins outright while the modifier is held, then
/// the Archive pill on row hover, then the status word or the time-ago.
function renderCorner(row: ChatRow, hovered: boolean, archived: boolean,
  jumpLabel: string | null, onSetArchived: (id: string, archived: boolean) => void) {
  if (jumpLabel) {
    return <span className="h-4 flex-none flex flex-row items-center px-1 rounded-sm bg-text-muted/10 text-ui-10 font-medium text-text-muted/88">{jumpLabel}</span>;
  }
  if (hovered) {
    return (
      <span onClick={() => onSetArchived(row.id, !archived)} data-archive={row.id}
        className="flex flex-row items-center gap-1 h-5 px-1 rounded-md bg-wash/10 cursor-pointer hover:bg-wash/14">
        <span className="size-3 flex-none text-text-muted">{renderIcon(archived ? 'archive-up-minimalistic' : 'archive-minimalistic')}</span>
        <span className="text-ui-10 text-text-muted">{archived ? 'Unarchive' : 'Archive'}</span>
      </span>
    );
  }
  const label = statusLabel(row.status, row.send);
  if (!label) {
    return <span className="text-ui-10 font-medium">{row.timeAgo}</span>;
  }
  const tone = statusTone(row.status, row.send);
  return (
    <span className="flex flex-row items-center gap-1">
      {row.status === 'completed' && row.send === 'normal'
        ? <span className="size-3 flex-none text-success/88">{renderIcon('check')}</span>
        : statusDot(tone)}
      {statusWord(tone, label)}
    </span>
  );
}

export type ChatRowCallbacks = {
  onOpenChat: (chatId: string) => void;
  onSetArchived: (chatId: string, archived: boolean) => void;
  onOpenChatMenu: (chatId: string) => void;
};

/// One active session card. Line 1 is "project @ device" with the status
/// corner, line 2 the harness mark and title, line 3 the optional branch and
/// pull-request badge. Line 3 is structural: compact rows omit it entirely.
/// shell.rs:5172 blends the row's text and wash through motion::hover_blend,
/// so the unselected branch fades. The selected branch's rest and hover fills
/// are the same colour in Rust, so it has nothing to fade and snaps.
export function renderChatRow(row: ChatRow, selected: boolean, hovered: boolean,
  showBranch: boolean, showPullRequest: boolean, showHarness: boolean,
  jumpLabel: string | null, onHover: (id: string | null) => void, cb: ChatRowCallbacks) {
  const branch = showBranch ? row.branch : null;
  const changeRequest = showPullRequest ? row.changeRequest : null;
  const showsMetadata = branch !== null || changeRequest !== null;
  return (
    <div key={row.id} data-chat={row.id} role="button" tabIndex={0}
      onClick={() => cb.onOpenChat(row.id)}
      onContextMenu={() => cb.onOpenChatMenu(row.id)}
      onMouseEnter={() => onHover(row.id)} onMouseLeave={() => onHover(null)}
      className={selected
        ? 'flex flex-col gap-0.5 rounded-lg px-2 py-1.5 text-text bg-wash/10 cursor-pointer focus:bg-wash/14'
        : 'flex flex-col gap-0.5 rounded-lg px-2 py-1.5 text-text/88 cursor-pointer motion-hover-fade hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'}>
      <div className="w-full flex flex-row items-center gap-2">
        <div className="flex-1 min-w-0 truncate whitespace-nowrap text-ui-11 text-text-muted/50">{row.folder}</div>
        <div className="flex-none flex items-center h-3.5 text-text-muted/50">
          {renderCorner(row, hovered, row.archived, jumpLabel, cb.onSetArchived)}
        </div>
      </div>
      <div className="w-full flex flex-row items-center gap-2">
        {showHarness && harnessMark(row.harness)}
        <div className="flex-1 min-w-0 truncate whitespace-nowrap text-ui-13">{row.title}</div>
      </div>
      {showsMetadata && (
        <div className="w-full flex flex-row items-center gap-1">
          {branch !== null && <span className="size-3 flex-none text-text-muted/50">{renderIcon('git-branch')}</span>}
          {branch !== null && <span className="min-w-0 truncate whitespace-nowrap text-ui-11 text-text-muted/50">{branch}</span>}
          <span className="flex-1 min-w-0" />
          {changeRequest !== null && pullRequestBadge(changeRequest.number, changeRequest.state)}
        </div>
      )}
    </div>
  );
}

/// The archived shelf's slim one-liner: 36px, dimmed mark at rest restored on
/// hover, title, and a right slot where the time yields to Unarchive.
export function renderArchivedRow(row: ChatRow, selected: boolean, hovered: boolean,
  showHarness: boolean, onHover: (id: string | null) => void, cb: ChatRowCallbacks) {
  return (
    <div key={row.id} data-archived={row.id} role="button" tabIndex={0}
      onClick={() => cb.onOpenChat(row.id)}
      onContextMenu={() => cb.onOpenChatMenu(row.id)}
      onMouseEnter={() => onHover(row.id)} onMouseLeave={() => onHover(null)}
      className={selected
        ? 'h-9 flex flex-row items-center gap-2.5 px-2 rounded-md cursor-pointer bg-wash/10 focus:bg-wash/14'
        : 'h-9 flex flex-row items-center gap-2.5 px-2 rounded-md cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
      {showHarness && harnessMark(row.harness)}
      <span className={hovered || selected
        ? 'flex-1 min-w-0 truncate whitespace-nowrap text-ui-13 text-text'
        : 'flex-1 min-w-0 truncate whitespace-nowrap text-ui-13 text-text/50'}>{row.title}</span>
      {hovered ? (
        <span onClick={() => cb.onSetArchived(row.id, false)} data-unarchive={row.id}
          className="flex flex-row items-center gap-1 h-5 px-1 rounded-md bg-wash/10 cursor-pointer hover:bg-wash/14">
          <span className="size-3 flex-none text-text-muted">{renderIcon('archive-up-minimalistic')}</span>
          <span className="text-ui-10 text-text-muted">Unarchive</span>
        </span>
      ) : <span className="text-ui-11 text-text-muted/50">{row.timeAgo}</span>}
    </div>
  );
}
