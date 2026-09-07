// Rust: app/crates/ui/src/settings/appearance.rs (AppearancePage::render). The
// Rust file is 2530 lines, so its render helpers and free functions are split
// into appearance/ files named for the Rust section each one mirrors.
import { useState } from 'react';
import type { ReactNode } from 'react';
import {
  ACCENTS, MOTION_MODES, SURFACES, accentHelper, accentSwatch, choiceChip,
  motionHelper, motionLabel, surfaceHelper, surfaceLabel,
} from './appearance/chips';
import { renderFontPicker, renderSizePicker } from './appearance/fonts';
import type { FontChoice } from './appearance/fonts';
import { renderImportDialog } from './appearance/import_dialog';
import type { ImportDialogState } from './appearance/import_dialog';
import { renderThemeLibraryRows } from './appearance/library';
import type { LibraryCallbacks } from './appearance/library';
import { preview } from './appearance/previews';
import { renderReviewDialog } from './appearance/review_dialog';
import { renderThemeSelector } from './appearance/theme_selector';
import {
  cardRow, errorStrip, fieldLabel, metaLine, optionCard, optionCardRow, pageColumn,
  pageHeader, pageSubtitle, rowTile, rowTitle, sectionCard,
} from './widgets';

export type AppearanceMode = 'system' | 'light' | 'dark';
export type MotionMode = 'system' | 'full' | 'reduced';
export type SurfacePreference = 'theme-default' | 'frosted' | 'opaque';
export type AccentChoice = 'theme-default' | 'Surya' | 'Orange' | 'Amber' | 'Green' | 'Cyan' | 'Blue' | 'Pink';
export type InstallMode = 'snapshot' | 'link';
export type ThemeVariantOption = { id: string; name: string; appearance: 'light' | 'dark' };
export type ImportReport = { summary: string; lines: readonly string[] };
export type CustomThemeEntry = {
  id: string; name: string; linked: boolean; status: 'ready' | 'warning';
  statusLine: string; variants: readonly ThemeVariantOption[];
  reports?: Readonly<Record<string, ImportReport>>;
};

export type AppearanceProps = LibraryCallbacks & {
  mode: AppearanceMode;
  themes: { light: string; dark: string };
  lightVariants: readonly ThemeVariantOption[];
  darkVariants: readonly ThemeVariantOption[];
  accent: AccentChoice;
  surface: SurfacePreference;
  resolvedSurface: 'frosted' | 'opaque';
  motion: MotionMode;
  systemReducedMotion: boolean;
  font: string;
  fontChoices: readonly FontChoice[];
  fontSize: string;
  fontSizes: readonly string[];
  fontFailed: boolean;
  library: readonly CustomThemeEntry[];
  libraryWarning?: string;
  onSetMode?: (mode: AppearanceMode) => void;
  onSetTheme?: (kind: 'light' | 'dark', variantId: string) => void;
  onSetAccent?: (accent: AccentChoice) => void;
  onSetSurface?: (surface: SurfacePreference) => void;
  onSetMotion?: (motion: MotionMode) => void;
  onSetFont?: (font: string) => void;
  onSetFontSize?: (size: string) => void;
};

const MODES: readonly AppearanceMode[] = ['system', 'light', 'dark'];

function modeLabel(mode: AppearanceMode) {
  if (mode === 'light') return 'Light';
  if (mode === 'dark') return 'Dark';
  return 'System';
}

// Rust: the settings_rows vector. One card row per setting, in the Rust order:
// light theme, dark theme, accent, glass, motion, then the library rows.
function settingRow(key: string, icon: 'tuning' | 'widget', title: string, helper: string, control: ReactNode) {
  return (
    <div key={key} data-row={key}>
      {cardRow(key === 'light-theme', (
        <>
          {rowTile(icon)}
          <div className="flex-1 min-w-0">
            {rowTitle(title)}
            {metaLine([helper])}
          </div>
          <div className="flex-none ml-2.5 flex items-center gap-1.5">{control}</div>
        </>
      ))}
    </div>
  );
}

export function AppearancePage(props: AppearanceProps) {
  const [openThemeMenu, setOpenThemeMenu] = useState<'light' | 'dark' | null>(null);
  const [fontMenuOpen, setFontMenuOpen] = useState(false);
  const [sizeMenuOpen, setSizeMenuOpen] = useState(false);
  const [reviewEntry, setReviewEntry] = useState<string | null>(null);
  const [importDialog, setImportDialog] = useState<ImportDialogState | null>(null);

  const reviewed = props.library.find(entry => entry.id === reviewEntry);

  const cards = MODES.map(mode => (
    <div key={mode} className="flex-1 min-w-0 flex">
      {optionCard(modeLabel(mode), mode === props.mode, preview(mode), () => props.onSetMode?.(mode))}
    </div>
  ));

  const rows: ReactNode[] = [
    settingRow('light-theme', 'tuning', 'Light theme', 'Used whenever this appearance is active.',
      renderThemeSelector('light', props.lightVariants, props.themes.light, openThemeMenu === 'light',
        () => setOpenThemeMenu(openThemeMenu === 'light' ? null : 'light'),
        variantId => { setOpenThemeMenu(null); props.onSetTheme?.('light', variantId); })),
    settingRow('dark-theme', 'tuning', 'Dark theme', 'Used whenever this appearance is active.',
      renderThemeSelector('dark', props.darkVariants, props.themes.dark, openThemeMenu === 'dark',
        () => setOpenThemeMenu(openThemeMenu === 'dark' ? null : 'dark'),
        variantId => { setOpenThemeMenu(null); props.onSetTheme?.('dark', variantId); })),
    settingRow('accent', 'tuning', 'Accent color', accentHelper(props.accent),
      ACCENTS.map(accent => accentSwatch(accent, accent === props.accent, () => props.onSetAccent?.(accent)))),
    settingRow('glass', 'widget', 'Glass', surfaceHelper(props.surface, props.resolvedSurface),
      SURFACES.map(surface => choiceChip(`surface-${surface}`, surfaceLabel(surface),
        surface === props.surface, () => props.onSetSurface?.(surface)))),
    settingRow('motion', 'tuning', 'Motion', motionHelper(props.motion, props.systemReducedMotion),
      MOTION_MODES.map(mode => choiceChip(`motion-${mode}`, motionLabel(mode),
        mode === props.motion, () => props.onSetMotion?.(mode)))),
    ...renderThemeLibraryRows(props.library, {
      ...props,
      onAddTheme: () => setImportDialog({ source: '', mode: 'snapshot', selected: [] }),
      onReviewTheme: id => setReviewEntry(id),
    }),
  ];

  return (
    <section aria-labelledby="appearance-heading" data-screen="appearance"
      className="size-full overflow-y-scroll relative">
      {pageColumn(
        <>
          <span id="appearance-heading">{pageHeader('Appearance')}</span>
          {pageSubtitle('Choose how Surya looks. These settings stay on this device.', true)}
          <div className="mt-8 flex flex-col gap-3">
            {fieldLabel('Appearance')}
            {optionCardRow(cards)}
          </div>
          {sectionCard(rows)}
          <div className="mt-9 flex flex-col gap-2.5">
            <div className="flex flex-row items-center justify-between gap-6">
              <div className="min-w-0 flex-1 flex flex-col gap-1">
                {fieldLabel('Interface font')}
                <p className="max-w-96 text-ui-12 leading-normal text-text-muted">
                  Used across the interface and conversations. Code, diffs, and terminal keep
                  their current fonts and sizes.
                </p>
              </div>
              <div className="flex-none flex flex-row items-center gap-2">
                {renderFontPicker(props.fontChoices, props.font, fontMenuOpen,
                  () => { setFontMenuOpen(!fontMenuOpen); setSizeMenuOpen(false); },
                  font => { setFontMenuOpen(false); props.onSetFont?.(font); })}
                {renderSizePicker(props.fontSizes, props.fontSize, sizeMenuOpen,
                  () => { setSizeMenuOpen(!sizeMenuOpen); setFontMenuOpen(false); },
                  size => { setSizeMenuOpen(false); props.onSetFontSize?.(size); })}
              </div>
            </div>
            {props.fontFailed && errorStrip('This font could not be loaded. Comet is using Geist.')}
          </div>
          {props.libraryWarning !== undefined && (
            <div className="mt-2 text-ui-11 text-warning">{props.libraryWarning}</div>
          )}
        </>,
      )}
      {importDialog !== null && renderImportDialog(importDialog, {
        onClose: () => setImportDialog(null),
        onSetMode: mode => setImportDialog({ ...importDialog, mode }),
        onToggleVariant: variantId => setImportDialog({
          ...importDialog,
          selected: importDialog.selected.includes(variantId)
            ? importDialog.selected.filter(id => id !== variantId)
            : [...importDialog.selected, variantId],
        }),
        onToggleReview: variantId => setImportDialog({
          ...importDialog,
          reviewVariant: importDialog.reviewVariant === variantId ? undefined : variantId,
        }),
      })}
      {reviewed !== undefined && renderReviewDialog(reviewed, () => setReviewEntry(null))}
    </section>
  );
}
