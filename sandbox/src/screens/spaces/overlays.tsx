// Rust: app/crates/ui/src/shell/spaces.rs render_space_overlays (2982) - the
// space row's right-click menu, the rename dialog, and the remove
// confirmation. Render helpers on the same pane.
import { btnDanger, btnGhost, btnPrimary, dialogBody, dialogCard, dialogTitle, menuRow, menuSeparator, modalScrim, paintedQuery, popoverCard } from './chrome';
import { renderIcon } from './icons';

/// The space context menu: Rename, a hairline, then Remove in danger tone.
/// The native menu is positioned at the pointer; here it anchors to the
/// spaces menu's own column.
export function renderSpaceMenu(onRename: () => void, onRemove: () => void) {
  return (
    <div className="absolute top-20 left-4 w-40" data-menu="space-context">
      {popoverCard(
        <div className="flex flex-col">
          {menuRow(false, false, onRename, 'space-menu-rename', (
            <>
              <span className="size-4 flex-none text-text-muted">{renderIcon('pen')}</span>
              <span>Rename...</span>
            </>
          ))}
          {menuSeparator('space-menu-sep')}
          {menuRow(false, false, onRemove, 'space-menu-delete', (
            <>
              <span className="size-4 flex-none text-danger">{renderIcon('trash-bin-minimalistic')}</span>
              <span>Remove...</span>
            </>
          ), true)}
        </div>
      )}
    </div>
  );
}

/// The rename dialog. Its field is a GPUI-painted ComposerInput; the sandbox
/// paints the current name and a caret rather than mounting a text input.
export function renderRenameSpaceDialog(name: string, onCancel: () => void, onSubmit: () => void) {
  return modalScrim(dialogCard(
    <>
      {dialogTitle('Rename project')}
      <div className="mt-3 w-full px-3 py-2 rounded-lg border border-border bg-wash/5 text-ui-14">
        {paintedQuery(name, 'Project name')}
      </div>
      <div className="mt-4 flex flex-row justify-end gap-2">
        {btnGhost('Cancel', onCancel)}
        {btnPrimary('Rename', onSubmit)}
      </div>
    </>
  ));
}

/// The remove confirmation. The copy names the space, its device and how many
/// sessions go with it, singular and plural exactly as the native does.
export function renderDeleteSpaceDialog(name: string, device: string, count: number,
  onCancel: () => void, onConfirm: () => void) {
  const copy = count === 1
    ? 'Removing "' + name + '" permanently deletes its 1 session on ' + device + '. This cannot be undone.'
    : 'Removing "' + name + '" permanently deletes its ' + String(count) + ' sessions on ' + device + '. This cannot be undone.';
  return modalScrim(dialogCard(
    <>
      {dialogTitle('Remove project?')}
      <div className="mt-1.5">{dialogBody(copy)}</div>
      <div className="mt-4 flex flex-row justify-end gap-2">
        {btnGhost('Cancel', onCancel)}
        {btnDanger('Remove', onConfirm)}
      </div>
    </>
  ));
}
