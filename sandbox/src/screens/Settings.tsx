// Rust: app/crates/ui/src/shell.rs (Shell::render_settings_nav and the
// Route::Settings branch of Shell::render_main). settings/window.rs holds the
// saved window rect and has no render tree, so nothing from it is ported.
// The section rail is the settings route's sidebar; the pane is the outlet.
import { useState } from 'react';
import type { ReactNode } from 'react';
import { settingsIcon } from './settings/widgets';
import type { SettingsIcon } from './settings/widgets';

export type SettingsSection = { id: string; label: string; icon: SettingsIcon };
export type SettingsProps = {
  sections: readonly SettingsSection[];
  section: string;
  renderSection: (id: string) => ReactNode;
  onOpenSection: (id: string) => void;
  onBack: () => void;
};

export function SettingsWindow({ sections, section, renderSection, onOpenSection, onBack }: SettingsProps) {
  const [current, setCurrent] = useState(section);
  function open(id: string) {
    setCurrent(id);
    onOpenSection(id);
  }
  return (
    <div className="size-full relative flex flex-row bg-surface text-text font-sans leading-gpui">
      <nav aria-label="Settings" className="w-64 h-full flex flex-col">
        <div className="flex-1 px-2 flex flex-col">
          <div className="px-2 pt-3 pb-1 text-ui-11 font-medium text-text-faint">Settings</div>
          <div className="flex flex-col gap-0.5">
            {sections.map(item => (
              <button key={item.id} type="button" data-section={item.id}
                aria-current={item.id === current ? 'page' : undefined}
                onClick={() => open(item.id)}
                className={item.id === current
                  ? 'flex flex-row items-center gap-2 rounded-lg px-2 py-1.5 text-ui-13 text-left cursor-pointer bg-wash/10 font-medium text-text hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'
                  : 'flex flex-row items-center gap-2 rounded-lg px-2 py-1.5 text-ui-13 text-left cursor-pointer text-text-muted hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'}>
                <span className="flex-none size-4 flex text-text-muted">{settingsIcon(item.icon)}</span>
                {item.label}
              </button>
            ))}
          </div>
        </div>
        <div className="px-2 pb-3">
          <button type="button" data-action="settings-back" onClick={onBack}
            className="w-full flex flex-row items-center gap-1.5 rounded-lg px-2 py-1.5 text-ui-13 text-left text-text-muted cursor-pointer hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover">
            <span className="flex-none size-4 flex text-text-muted">{settingsIcon('alt-arrow-left')}</span>
            Back
          </button>
        </div>
      </nav>
      <main className="flex-1 min-w-0 h-full pt-9.5 flex flex-col">
        <div className="flex-1 min-h-0">{renderSection(current)}</div>
      </main>
    </div>
  );
}
