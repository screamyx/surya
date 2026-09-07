// Rust: app/crates/ui/src/settings/accounts.rs (AccountsPage::render, plus its
// render_device_switcher, render_usage_meter, render_account_row,
// render_login_dialog and render_skeleton_row helpers). The Rust file is one
// module, so this is one file; the popover, dialog and skeleton primitives it
// calls live in app/crates/ui/src/popover.rs.
import { useState } from 'react';
import type { ReactNode } from 'react';
import { pageIcon } from './appearance/icons';
import { trackedUpper } from './appearance/chips';
import {
  badge, badgeActive, errorStrip, ghostAction, pageColumn, pageHeader, pageSubtitle,
  sectionCard, settingsIcon, warningStrip,
} from './widgets';

export type ProviderId = 'claude-code' | 'codex' | 'cursor';
export type UsageWindow = { label: string; usedFraction: number; reset?: string };
export type AgentAccount = {
  id: string; harness: ProviderId; email: string; planLabel?: string;
  active: boolean; switchable: boolean; usageWindows: readonly UsageWindow[];
};
export type AccountWarning = { harness: ProviderId; message: string };
export type DeviceRow = { id: string; name: string; platform: string };
export type LoginFlow =
  | { kind: 'starting'; harness: ProviderId }
  | { kind: 'pasteCode'; harness: ProviderId; url: string; code: string; submitting: boolean; error?: string }
  | { kind: 'browser'; harness: ProviderId; url: string; message?: string; error?: string };

export type AccountsProps = {
  status: 'loading' | 'ready' | 'error';
  accounts: readonly AgentAccount[];
  warnings: readonly AccountWarning[];
  devices: readonly DeviceRow[];
  localDeviceId: string;
  busyAccountId?: string;
  actionError?: string;
  loadError?: string;
  login?: LoginFlow;
  onRefresh?: () => void;
  onRetryLoad?: () => void;
  onDismissError?: () => void;
  onRetargetDevice?: (deviceId: string | null) => void;
  onAddAccount?: (harness: ProviderId) => void;
  onSwitchAccount?: (accountId: string) => void;
  onForgetAccount?: (accountId: string) => void;
  onOpenLoginUrl?: (url: string) => void;
  onSubmitCode?: () => void;
  onCancelLogin?: () => void;
};

// Rust: PROVIDERS. Display order and the CLI named in the empty-state copy.
const PROVIDERS: readonly { id: ProviderId; name: string; cli: string }[] = [
  { id: 'claude-code', name: 'Claude Code', cli: 'claude' },
  { id: 'codex', name: 'Codex', cli: 'codex' },
  { id: 'cursor', name: 'Cursor', cli: 'cursor-agent' },
];

// Rust: USAGE_WARN_FRACTION and USAGE_CRITICAL_FRACTION.
const USAGE_WARN = 0.8;
const USAGE_CRITICAL = 0.95;

// Rust: render_device_switcher's platform_glyph.
function platformGlyph(platform: string) {
  if (platform === 'macos' || platform === 'darwin') return 'laptop' as const;
  if (platform === 'ios' || platform === 'android') return 'smartphone' as const;
  return 'monitor' as const;
}

// Rust: render_usage_meter's fill, `.w(relative(fraction.max(0.015)))` painted
// in usage_color(level).opacity(..). The whitelist carries no fractional widths,
// so the fill box is quantized to the admitted `w-` step nearest
// `fraction * 220px`, the track's max width, and the tone rides inside it.
function meterFill(fraction: number, percent: number) {
  return (
    <div data-fill={percent} className={fraction >= 0.936 ? 'w-55 flex-none h-full'
      : fraction >= 0.8 ? 'w-48 flex-none h-full'
      : fraction >= 0.655 ? 'w-40 flex-none h-full'
      : fraction >= 0.545 ? 'w-32 flex-none h-full'
      : fraction >= 0.473 ? 'w-28 flex-none h-full'
      : fraction >= 0.4 ? 'w-24 flex-none h-full'
      : fraction >= 0.327 ? 'w-20 flex-none h-full'
      : fraction >= 0.255 ? 'w-16 flex-none h-full'
      : fraction >= 0.2 ? 'w-12 flex-none h-full'
      : fraction >= 0.164 ? 'w-10 flex-none h-full'
      : fraction >= 0.127 ? 'w-8 flex-none h-full'
      : fraction >= 0.091 ? 'w-6 flex-none h-full'
      : fraction >= 0.055 ? 'w-4 flex-none h-full'
      : fraction >= 0.027 ? 'w-2 flex-none h-full'
      : 'w-1 flex-none h-full'}>
      <div className={fraction >= USAGE_CRITICAL ? 'size-full rounded-full bg-danger/88'
        : fraction >= USAGE_WARN ? 'size-full rounded-full bg-warning/88'
        : 'size-full rounded-full bg-accent/88'} />
    </div>
  );
}

// Rust: render_usage_meter. Label, 5px bar, "NN% used", quiet reset time.
function renderUsageMeter(usage: UsageWindow) {
  const percent = Math.round(Math.min(Math.max(usage.usedFraction, 0), 1) * 100);
  return (
    <div key={usage.label} className="flex flex-row items-center gap-2 text-ui-11 text-text-faint">
      <div className="w-12 flex-none truncate">{usage.label}</div>
      <div className="flex-1 min-w-14 max-w-55 h-1 rounded-full overflow-hidden bg-wash/5 flex flex-row">
        {usage.usedFraction > 0 && meterFill(usage.usedFraction, percent)}
      </div>
      <div className="w-16 flex-none text-right">{percent}% used</div>
      {usage.reset !== undefined && <div className="flex-none truncate text-text-faint/50">{usage.reset}</div>}
    </div>
  );
}

// Rust: render_account_row. Initial avatar and meters left, badges over the
// Switch/Forget actions right. Actions appear on inactive accounts only.
function renderAccountRow(
  account: AgentAccount, first: boolean, busy: boolean,
  onSwitch?: (id: string) => void, onForget?: (id: string) => void,
) {
  const initial = account.email.slice(0, 1).toUpperCase();
  return (
    <div key={account.id} data-account={account.id}
      className={first
        ? 'px-5 py-3.5 flex flex-row items-stretch gap-3'
        : 'px-5 py-3.5 border-t border-border flex flex-row items-stretch gap-3'}>
      <div className="flex-none self-center size-8 rounded-full border border-border bg-wash/5 flex items-center justify-center text-ui-12 font-semibold text-text-muted">
        {initial}
      </div>
      <div className="flex-1 min-w-0 flex flex-col">
        <div className="min-w-0 truncate text-ui-13 font-medium text-text">{account.email}</div>
        {account.usageWindows.length === 0 ? (
          <div className="mt-1.5 truncate text-ui-11 text-text-faint">
            {account.switchable ? 'Usage unavailable' : 'Credentials unavailable'}
          </div>
        ) : (
          <div className="mt-1.5 flex flex-col gap-1">{account.usageWindows.map(renderUsageMeter)}</div>
        )}
      </div>
      <div className="flex-none flex flex-col items-end justify-between gap-2">
        <div className="flex flex-row items-center gap-1.5">
          {account.active && badgeActive('Active')}
          {account.planLabel !== undefined && badge(account.planLabel)}
        </div>
        {!account.active && (
          <div className="flex flex-row items-center gap-1">
            <button type="button" aria-label={`Forget ${account.email}`} data-forget={account.id}
              onClick={() => onForget?.(account.id)}
              className={busy
                ? 'rounded-md px-1.5 py-1 text-text-muted cursor-pointer opacity-50'
                : 'rounded-md px-1.5 py-1 text-text-muted cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5 focus:text-text'}>
              <span className="size-3.5 flex">{settingsIcon('trash-bin-minimalistic')}</span>
            </button>
            {account.switchable && (
              <button type="button" data-switch={account.id} onClick={() => onSwitch?.(account.id)}
                className={busy
                  ? 'px-2 py-1 rounded-md bg-text text-ui-11 font-medium text-on-solid cursor-pointer opacity-50'
                  : 'px-2 py-1 rounded-md bg-text text-ui-11 font-medium text-on-solid cursor-pointer hover:bg-text/88 active:bg-text/50 focus:bg-text/88'}>
                {busy ? 'Switching…' : 'Switch'}
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// Rust: loaders::gradient_spinner("login-poll", theme, 3.0, ..). A 3x3 grid of
// round cells. Each cell's phase is its distance from the bottom-centre origin,
// d = 2 - row + |col - 1|, so the pulse travels upward; motion-gradient-spin-<d>
// is that same offset as a negative animation delay. The native per-row tints
// are raw hex in proto::motion::GSPIN_ROW_TINTS, which the sandbox has no token
// for, so every cell paints the accent instead.
function gradientSpinner() {
  return (
    <div className="flex-none flex flex-col gap-0.5">
      <div className="flex flex-row gap-0.5">
        <div className="size-0.75 rounded-full bg-accent motion-gradient-spin-3" />
        <div className="size-0.75 rounded-full bg-accent motion-gradient-spin-2" />
        <div className="size-0.75 rounded-full bg-accent motion-gradient-spin-3" />
      </div>
      <div className="flex flex-row gap-0.5">
        <div className="size-0.75 rounded-full bg-accent motion-gradient-spin-2" />
        <div className="size-0.75 rounded-full bg-accent motion-gradient-spin-1" />
        <div className="size-0.75 rounded-full bg-accent motion-gradient-spin-2" />
      </div>
      <div className="flex flex-row gap-0.5">
        <div className="size-0.75 rounded-full bg-accent motion-gradient-spin-1" />
        <div className="size-0.75 rounded-full bg-accent motion-gradient-spin-0" />
        <div className="size-0.75 rounded-full bg-accent motion-gradient-spin-1" />
      </div>
    </div>
  );
}

// Rust: render_skeleton_row. The same geometry as a real row so loaded data
// lands without a layout jump; row two is dimmed. accounts.rs:1196 pulses the
// inner box at opacity 0.55 + 0.35 * wave on SURYA_PULSE, with no stagger and
// no size change. motion-surya-pulse-* is the loaders.rs cell, 0.08 to 1 plus
// 90% to 100% size, so it would paint a different animation. This renders at
// its resting opacity.
function renderSkeletonRow(key: string, dim: boolean, first: boolean) {
  const ghost = <div className="flex-1 min-w-14 max-w-55 h-1 rounded-full bg-wash/5" />;
  return (
    <div key={key} data-skeleton={key}
      className={dim
        ? (first ? 'px-5 py-3.5 opacity-50' : 'px-5 py-3.5 border-t border-border opacity-50')
        : (first ? 'px-5 py-3.5' : 'px-5 py-3.5 border-t border-border')}>
      <div className="opacity-55 flex flex-row items-stretch gap-3">
        <div className="flex-none self-center size-8 rounded-full bg-wash/5" />
        <div className="flex-1 min-w-0">
          <div className="w-40 max-w-full h-3 rounded-sm bg-wash/5" />
          <div className="mt-2 flex flex-col gap-2">
            <div className="flex flex-row items-center gap-2">
              <div className="w-12 flex-none h-2.25 rounded-sm bg-wash/5" />{ghost}
              <div className="w-16 flex-none h-2.25 rounded-sm bg-wash/5" />
            </div>
            <div className="flex flex-row items-center gap-2">
              <div className="w-12 flex-none h-2.25 rounded-sm bg-wash/5" />{ghost}
              <div className="w-16 flex-none h-2.25 rounded-sm bg-wash/5" />
            </div>
          </div>
        </div>
        <div className="flex-none flex flex-col items-end">
          <div className="w-16 h-5 rounded-full bg-wash/5" />
        </div>
      </div>
    </div>
  );
}

// Rust: LoginFlow::title.
function loginTitle(harness: ProviderId) {
  if (harness === 'codex') return 'Add Codex account';
  if (harness === 'cursor') return 'Connect Cursor';
  return 'Add Claude account';
}

// Rust: render_login_dialog. The paste-code and browser-poll flows in the
// popover::modal scrim; popover::dialog_card is the 360px card.
function renderLoginDialog(props: AccountsProps, login: LoginFlow) {
  const urlLink = (label: string, url: string) => (
    <button type="button" onClick={() => props.onOpenLoginUrl?.(url)} data-open-url={url}
      className="mt-1.5 text-ui-12 text-text-faint truncate text-left cursor-pointer hover:text-text active:text-text-muted focus:text-text">
      {label}
    </button>
  );
  let body: ReactNode = null;
  if (login.kind === 'starting') {
    body = (
      <div className="mt-2 flex flex-col gap-2">
        <div className="h-7 rounded-md bg-wash/5" />
        <div className="h-7 rounded-md bg-wash/5" />
      </div>
    );
  } else if (login.kind === 'pasteCode') {
    body = (
      <div className="flex flex-col">
        <p className="mt-2 text-ui-13 leading-normal text-text-muted">
          A browser window opened. Sign in to the account you want to add, approve access, then
          paste the code Anthropic shows you below. Your current login is untouched until you switch.
        </p>
        {urlLink('Reopen the authorization page', login.url)}
        <div className="mt-3 w-full px-3 py-2 rounded-lg border border-border bg-wash/5 text-ui-14 text-text">
          <span className="text-text-faint">{login.code === '' ? 'Paste the code' : login.code}</span>
        </div>
        {login.error !== undefined && <div className="mt-2 text-ui-12 text-danger-muted/88">{login.error}</div>}
        <div className="mt-4 flex flex-row justify-end gap-2">
          <button type="button" onClick={() => props.onCancelLogin?.()} data-action="login-cancel"
            className="motion-hover-fade px-3 py-1.5 rounded-lg text-ui-13 text-text-muted cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5">
            Cancel
          </button>
          <button type="button" onClick={() => props.onSubmitCode?.()} data-action="login-submit-code"
            className={login.submitting
              ? 'px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer opacity-50'
              : 'px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer hover:bg-text/88 active:bg-text/50 focus:bg-text/88'}>
            {login.submitting ? 'Verifying…' : 'Add account'}
          </button>
        </div>
      </div>
    );
  } else {
    body = (
      <div className="flex flex-col">
        <p className="mt-2 text-ui-13 leading-normal text-text-muted">
          {login.harness === 'cursor'
            ? 'Finish signing in to Cursor in your browser. This mints a surya-named API key you can revoke any time from Cursor’s dashboard. It is separate from cursor-agent login.'
            : 'Finish signing in to OpenAI in your browser. The new login is captured in an isolated profile, so your current session is untouched until you switch.'}
        </p>
        {urlLink('Reopen the sign-in page', login.url)}
        {login.error === undefined && (
          <div className="mt-4 flex flex-row items-center gap-2">
            {gradientSpinner()}
            <div className="text-ui-12 text-text-faint">{login.message ?? 'Waiting for the browser…'}</div>
          </div>
        )}
        {login.error !== undefined && <div className="mt-3 text-ui-12 text-danger-muted/88">{login.error}</div>}
        <div className="mt-4 flex flex-row justify-end">
          <button type="button" onClick={() => props.onCancelLogin?.()} data-action="login-cancel"
            className="motion-hover-fade px-3 py-1.5 rounded-lg text-ui-13 text-text-muted cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5">
            {login.error === undefined ? 'Cancel' : 'Close'}
          </button>
        </div>
      </div>
    );
  }
  return (
    <div className="absolute top-0 left-0 size-full bg-bg/50 flex items-center justify-center">
      <div role="dialog" aria-label={loginTitle(login.harness)}
        className="relative motion-dialog-in w-96 p-5 rounded-xl bg-surface-dialog border border-border flex flex-col text-text">
        <div className="text-ui-16 font-semibold text-text">{loginTitle(login.harness)}</div>
        {body}
      </div>
    </div>
  );
}

export function AccountsPage(props: AccountsProps) {
  const [deviceMenuOpen, setDeviceMenuOpen] = useState(false);
  const [targetDevice, setTargetDevice] = useState<string | null>(null);
  const effective = targetDevice ?? props.localDeviceId;
  const selected = props.devices.find(device => device.id === effective);
  const accountCount = props.accounts.length > 0 ? props.accounts.length : undefined;

  // Rust: render_device_switcher. Platform glyph, name, presence dot, sort glyph.
  const deviceSwitcher = (
    <div className="relative flex-none flex flex-col">
      <button type="button" data-action="accounts-device-switcher" aria-expanded={deviceMenuOpen}
        onClick={() => setDeviceMenuOpen(!deviceMenuOpen)}
        className={deviceMenuOpen
          ? 'flex-none h-7 px-2 rounded-md flex flex-row items-center gap-1.5 cursor-pointer bg-wash/5'
          : 'flex-none h-7 px-2 rounded-md flex flex-row items-center gap-1.5 cursor-pointer hover:bg-wash/5 active:bg-wash/10 focus:bg-wash/5'}>
        <span className="size-4 flex-none flex text-text-muted">
          {settingsIcon(platformGlyph(selected?.platform ?? 'macos'))}
        </span>
        <span className="min-w-0 truncate text-ui-12 font-medium text-text">{selected?.name ?? 'This device'}</span>
        <span className={effective === props.localDeviceId
          ? 'size-1.5 rounded-full flex-none bg-success'
          : 'size-1.5 rounded-full flex-none bg-wash/14'} />
        <span className={deviceMenuOpen
          ? 'size-3.5 flex-none flex text-text-muted'
          : 'size-3.5 flex-none flex text-text-faint/50'}>{pageIcon('sort-vertical')}</span>
      </button>
      {deviceMenuOpen && (
        <div className="absolute top-7 right-0">
        <div className="relative motion-menu-in w-55 p-1 rounded-lg border border-border bg-surface-overlay flex flex-col gap-0.5 text-ui-13 text-text">
          <div className="px-2 pb-1 pt-1.5 text-ui-10 font-medium text-text-faint">{trackedUpper('Devices')}</div>
          {props.devices.map(device => {
            const isActive = device.id === effective;
            const isLocal = device.id === props.localDeviceId;
            return (
              <button key={device.id} type="button" data-device={device.id}
                onClick={() => {
                  setTargetDevice(isLocal ? null : device.id);
                  setDeviceMenuOpen(false);
                  props.onRetargetDevice?.(isLocal ? null : device.id);
                }}
                className={isActive
                  ? 'flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer bg-element-active text-text'
                  : 'motion-hover-fade flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer text-text-muted hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'}>
                <span className="size-4 flex-none flex text-text-muted">{settingsIcon(platformGlyph(device.platform))}</span>
                <span className="flex-1 min-w-0 truncate">{device.name}</span>
                {isLocal && <span className="flex-none text-ui-10 text-text-faint/50">You</span>}
                <span className={isLocal ? 'size-1.5 rounded-full flex-none bg-success' : 'size-1.5 rounded-full flex-none bg-wash/14'} />
              </button>
            );
          })}
        </div>
        </div>
      )}
    </div>
  );

  // Rust: the `sections` match on Loadable. One section per provider.
  let sections: ReactNode;
  if (props.status === 'loading') {
    sections = PROVIDERS.map(provider => (
      <div key={provider.id} className="mt-6 flex flex-col">
        <div className="flex flex-row items-center gap-2">
          <div className="flex-none size-6 flex items-center justify-center">
            <span className="size-4 flex text-text-muted">{pageIcon(providerMark(provider.id))}</span>
          </div>
          <div className="text-ui-14 font-medium text-text">{provider.name}</div>
        </div>
        <div className="mt-2 rounded-xl border border-border bg-surface-card overflow-hidden flex flex-col">
          {renderSkeletonRow(`${provider.id}-0`, false, true)}
          {renderSkeletonRow(`${provider.id}-1`, true, false)}
        </div>
      </div>
    ));
  } else if (props.status === 'error') {
    sections = loadErrorStrip(props.loadError ?? 'The accounts could not be listed.', props.onRetryLoad);
  } else {
    sections = PROVIDERS.map(provider => {
      const accounts = props.accounts.filter(account => account.harness === provider.id);
      const warnings = props.warnings.filter(warning => warning.harness === provider.id);
      return (
        <div key={provider.id} className="mt-6 flex flex-col">
          <div className="flex flex-row items-center gap-2">
            <div className="flex-none size-6 flex items-center justify-center">
              <span className={provider.id === 'claude-code' ? 'size-4 flex text-claude-brand' : 'size-4 flex text-text-muted'}>
                {pageIcon(providerMark(provider.id))}
              </span>
            </div>
            <div className="text-ui-14 font-medium text-text">{provider.name}</div>
            <div className="flex-1" />
            {ghostActionWithIcon('Add account', () => props.onAddAccount?.(provider.id))}
          </div>
          {warnings.map(warning => <div key={warning.message}>{warningStrip(warning.message)}</div>)}
          <div className="mt-2 rounded-xl border border-border bg-surface-card overflow-hidden flex flex-col">
            {accounts.length === 0 ? (
              <div className="px-5 py-8 text-center text-ui-14 text-text-faint">
                {provider.id === 'cursor'
                  ? `${provider.name} isn’t connected on this device. Connect it to run Cursor sessions.`
                  : `No ${provider.name} login detected on this device. Sign in with “${provider.cli}” or add an account.`}
              </div>
            ) : accounts.map((account, index) => renderAccountRow(
              account, index === 0, props.busyAccountId === account.id,
              props.onSwitchAccount, props.onForgetAccount,
            ))}
          </div>
        </div>
      );
    });
  }

  return (
    <section aria-labelledby="accounts-heading" data-screen="accounts"
      className="size-full overflow-y-scroll relative">
      {pageColumn(
        <>
          <div className="flex flex-row items-center gap-2.5">
            <span id="accounts-heading">{pageHeader('Accounts', accountCount)}</span>
            <div className="flex-1" />
            {ghostAction('Refresh', 'refresh', props.onRefresh, props.status === 'loading')}
            {deviceSwitcher}
          </div>
          {pageSubtitle('The Claude Code, Codex, and Cursor logins on this device. Surya detects the live session, keeps each account backed up, and can swap between them.')}
          {props.actionError !== undefined && errorStrip(props.actionError, props.onDismissError)}
          {sections}
          <p className="mt-6 text-ui-12 leading-normal text-text-faint">
            Switching rewrites the CLI’s stored login, so new agent sessions use the selected
            account immediately. On macOS, an already-running Claude Code can hold the previous
            login for up to ~30 seconds (Keychain cache).
          </p>
        </>,
      )}
      {props.login !== undefined && renderLoginDialog(props, props.login)}
    </section>
  );
}

// Rust: the Loadable::Error arm. The shared kit's errorStrip takes a message
// only; the native load error carries a second "Click to retry" line inside the
// same clickable strip, so this page renders that one itself.
function loadErrorStrip(message: string, onRetry?: () => void) {
  return (
    <button type="button" onClick={onRetry} data-action="accounts-load-error"
      className="mt-4 px-4 py-3 rounded-xl border border-danger/14 bg-danger/5 text-ui-12 text-danger-muted/88 flex flex-row items-start gap-2 w-full text-left cursor-pointer hover:bg-danger/10 active:bg-danger/14 focus:bg-danger/10">
      <span className="flex-none mt-0.5 size-4 flex">{settingsIcon('danger-triangle')}</span>
      <span className="min-w-0 flex flex-col">
        <span>{message}</span>
        <span className="mt-1 text-ui-11 text-text-muted">Click to retry</span>
      </span>
    </button>
  );
}

// Rust: the provider_icon closure in AccountsPage::render.
function providerMark(harness: ProviderId) {
  if (harness === 'codex') return 'openai-mark' as const;
  if (harness === 'cursor') return 'cursor-mark' as const;
  return 'claude-mark' as const;
}

// Rust: widgets::ghost_action with the ADD_CIRCLE leading icon. The shared kit's
// ghostAction only takes an icon from its own map, which has no add-circle.
function ghostActionWithIcon(label: string, onClick?: () => void) {
  return (
    <button type="button" onClick={onClick} data-action={label}
      className="flex-none flex flex-row items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-ui-12 text-text-muted cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5 focus:text-text">
      <span className="flex-none size-4 flex">{pageIcon('add-circle')}</span>
      {label}
    </button>
  );
}
