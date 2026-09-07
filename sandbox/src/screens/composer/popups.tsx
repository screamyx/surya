// Rust: app/crates/ui/src/composer.rs, Composer::render_file_mention_popup
// (4323) and Composer::render_slash_popup (4631). Both span the full pill
// width and share the same card shape; the @ and / tokens are exclusive.
import type { MentionResult, SlashCommand } from '../../fixtures/composer';
import { renderIcon } from '../../icons';
import { menuPopover, menuRow, skeletonRows } from '../popover';

function errorNote(message: string) {
  return <div className="px-3 py-2.5 text-ui-12 text-danger-muted">{message}</div>;
}

function note(message: string) {
  return <div className="px-3 py-2.5 text-ui-12 text-text-muted">{message}</div>;
}

export function renderFileMentionPopup(
  results: readonly MentionResult[],
  loading: boolean,
  error: string | null,
  active: number,
  onActive: (index: number) => void,
  onAccept: (path: string) => void,
) {
  if (loading && results.length === 0) return menuPopover(skeletonRows(3));
  if (error !== null) return menuPopover(errorNote(error));
  if (results.length === 0) return menuPopover(note('No matching files'));
  return menuPopover(
    <div data-list="mention" className="max-h-80 flex flex-col overflow-y-scroll">
      {results.map((result, index) => menuRow(
        index === active, false,
        () => { onActive(index); onAccept(result.path); },
        `file-mention-result-${index}`,
        <span className="w-full flex flex-row items-center gap-2">
          <span className="size-3.5 flex-none text-text-muted">
            {result.isDir ? renderIcon('folder') : renderIcon('document')}
          </span>
          <span className="flex-none text-ui-13 text-text">{result.name}</span>
          <span className="min-w-0 flex-1 truncate overflow-hidden text-ui-12 text-text-muted">{result.directory}</span>
        </span>,
      ))}
    </div>,
  );
}

export function renderSlashPopup(
  commands: readonly SlashCommand[],
  loading: boolean,
  error: string | null,
  active: number,
  onActive: (index: number) => void,
  onAccept: (name: string) => void,
) {
  if (loading && commands.length === 0) return menuPopover(skeletonRows(3));
  if (error !== null) return menuPopover(errorNote(error));
  if (commands.length === 0) return menuPopover(note('This agent has no slash commands'));
  return menuPopover(
    <div data-list="slash" className="max-h-80 flex flex-col overflow-y-scroll">
      {commands.map((command, index) => menuRow(
        index === active, false,
        () => { onActive(index); onAccept(command.name); },
        `slash-result-${index}`,
        <span className="w-full flex flex-row items-center gap-2">
          <span className="size-3.5 flex-none text-text-muted">{renderIcon('command')}</span>
          <span className="flex-none text-ui-12 font-medium text-text">/{command.name}</span>
          <span className="min-w-0 flex-1 truncate overflow-hidden text-ui-12 text-text-muted">{command.description}</span>
        </span>,
      ))}
    </div>,
  );
}
