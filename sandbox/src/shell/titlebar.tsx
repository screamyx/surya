// Rust: shell.rs render_title_bar / render_titlebar_cluster / caption controls.
import type { Destination } from '../shell';
import type { Fixture } from '../fixtures';
import { renderIcon } from '../icons';
export function iconButton(icon: Parameters<typeof renderIcon>[0], label: string,
  act: ((value: string) => void) | (() => void)) {
  return <button key={label} type="button" aria-label={label} title={label} onClick={() => act(label)}
    className="size-6 flex-none flex items-center justify-center rounded-md text-text-muted cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
    <span className="size-4">{renderIcon(icon)}</span>
  </button>;
}
export function renderTitlebar(fixture: Fixture, sidebarOpen: boolean, onEvent: (value: string) => void,
  onNavigate: (to: Destination) => void) {
  return <header className="absolute top-0 left-0 right-0 h-9.5 flex flex-row items-center pt-0.5 px-2.5">
    <div className={sidebarOpen ? 'w-64 flex-none flex flex-row items-center gap-2' : 'flex-none flex flex-row items-center gap-2'}>
      <div className="flex items-center gap-0.5">{iconButton('sidebar-minimalistic-left', 'Toggle sidebar', onEvent)}
      {iconButton('global', 'Open browser', () => onNavigate('browser'))}</div>
      <div className="flex items-center gap-0.5">{iconButton('arrow-left', 'Back', onEvent)}
      {iconButton('arrow-right', 'Forward', onEvent)}</div>
      {iconButton('plus', 'New chat', onEvent)}
    </div>
    <div className="flex-1 min-w-0 flex flex-row items-center gap-1.5 pl-1.5 text-ui-12 text-text-faint">
      <span className="size-3.5 text-claude-brand">{renderIcon('claude-mark')}</span>
      <span className="truncate">{fixture === 'seeded' ? 'surya @ WINBOX' : 'luvus @ devbox'}</span>
    </div>
    <div className="flex flex-row items-center gap-3">
      {iconButton('sidebar-minimalistic', 'Toggle right pane', () => onNavigate('changes'))}
      {iconButton('minus', 'Minimize preview', onEvent)}
      {iconButton('square', 'Maximize preview', onEvent)}
      {iconButton('close', 'Close preview', onEvent)}
    </div>
  </header>;
}
