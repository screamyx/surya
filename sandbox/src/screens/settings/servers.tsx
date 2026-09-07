// Rust: app/crates/ui/src/settings/servers.rs (ServersPage::render, render_row)
// and settings/servers/add.rs (the Add server dialog). settings/servers/active.rs
// is pure decision code, so its result arrives as the `active` prop rather than
// being recomputed here. The shell owns the engine swap and persists the list;
// this page only reports.
import { useState } from 'react';
import type { ReactNode } from 'react';
import { pageColumn, pageHeader, pageSubtitle, fieldLabel, sectionCard, cardRow, rowTile,
  rowTitle, metaLine, badgeActive, ghostAction, errorStrip } from './widgets';
import type { SettingsIcon } from './widgets';

export type ServerRow = { id: string; name: string; host: string; port: number; tokenSet: boolean };
// Rust: settings/servers/active.rs ActiveRow.
export type ActiveRow =
  | { kind: 'local' }
  | { kind: 'saved'; id: string }
  | { kind: 'commandLine'; url: string };
export type ServersProps = {
  servers: readonly ServerRow[];
  active: ActiveRow;
  status: { text: string; isError: boolean };
  addError: string | null;
  onConnect: (id: string | null) => void;
  onRemove: (id: string) => void;
  onSaveCommandLine: () => void;
  onAddServer: (name: string, host: string, port: string, token: string) => void;
};

// Rust: ServersPage::render_row. `tail` is the RowTail enum.
function renderRow(first: boolean, title: string, meta: readonly string[], icon: SettingsIcon,
  tail: ReactNode, remove: ReactNode) {
  return cardRow(first, (
    <>
      {rowTile(icon)}
      <div className="flex-1 min-w-0 flex flex-col">
        {rowTitle(title)}
        {metaLine(meta)}
      </div>
      {tail}
      {remove}
    </>
  ));
}

// Rust: settings/servers/add.rs render(). popover::dialog_field wraps a
// ComposerInput; here the field is a plain controlled input.
function dialogField(label: string, value: string, placeholder: string, onChange: (next: string) => void) {
  return (
    <div className="mt-3 flex flex-col gap-1.5">
      {fieldLabel(label)}
      <input type="text" value={value} placeholder={placeholder} aria-label={label}
        onChange={event => onChange(event.target.value)}
        className="w-full px-3 py-2 rounded-lg border border-border bg-wash/5 text-ui-14 text-text focus:border-border-strong" />
    </div>
  );
}

export function ServersPage({ servers, active, status, addError, onConnect, onRemove,
  onSaveCommandLine, onAddServer }: ServersProps) {
  const [addOpen, setAddOpen] = useState(false);
  const [name, setName] = useState('');
  const [host, setHost] = useState('');
  const [port, setPort] = useState('');
  const [token, setToken] = useState('');
  const count = servers.length;
  const connect = (label: string, id: string | null) =>
    ghostAction(label, 'global', () => onConnect(id));
  return (
    <div className="size-full overflow-y-scroll" data-page="servers">
      {pageColumn(
        <>
          {pageHeader('Servers', count > 0 ? count : undefined)}
          {pageSubtitle('Engines on other machines this app drives directly over your network. Sessions, files and terminals then live on that machine.')}
          <div className={status.isError ? 'mt-3 text-ui-13 text-danger' : 'mt-3 text-ui-13 text-text-muted'}>{status.text}</div>
          {sectionCard(
            <>
              {renderRow(true, 'This computer',
                ['Engine started by this app, or a daemon on the local port'], 'monitor',
                active.kind === 'local' ? badgeActive('Active') : connect('Connect', null), null)}
              {servers.map(server => renderRow(false, server.name,
                [`${server.host}:${server.port}`, server.tokenSet ? 'token set' : 'no token'], 'global',
                active.kind === 'saved' && active.id === server.id
                  ? badgeActive('Active')
                  : connect('Connect', server.id),
                ghostAction('Remove', 'trash-bin-minimalistic', () => onRemove(server.id), true)))}
              {active.kind === 'commandLine' && renderRow(false, 'Command line',
                [active.url, 'from --engine or SURYA_ENGINE; Save keeps it'], 'global',
                <>
                  {badgeActive('Active')}
                  {ghostAction('Save', 'global', onSaveCommandLine)}
                </>, null)}
            </>
          )}
          <div className="mt-4 flex flex-row">
            <button type="button" data-action="add-server" onClick={() => setAddOpen(true)}
              className="px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer hover:bg-text/88 active:bg-text/50 focus:bg-text/88">Add server</button>
          </div>
        </>
      )}
      {addOpen && (
        <div className="absolute top-0 left-0 size-full flex items-center justify-center bg-bg/50">
          <div role="dialog" aria-label="Add server"
            className="w-96 p-5 rounded-xl border border-border bg-surface-dialog flex flex-col text-text">
            <div className="text-ui-14 font-semibold text-text">Add server</div>
            <div className="mt-1.5 text-ui-13 text-text-muted leading-normal">The other machine runs `surya headless --bind &lt;its address&gt;`. `surya status` there prints the token.</div>
            {dialogField('Name', name, 'Name (optional)', setName)}
            {dialogField('Host', host, 'Host, e.g. 100.64.0.9 or build-box', setHost)}
            {dialogField('Port', port, 'Port (default 27700)', setPort)}
            {dialogField('Token', token, 'Token from `surya status` on that machine', setToken)}
            {addError !== null && errorStrip(addError)}
            <div className="mt-4 flex flex-row justify-end gap-2">
              <button type="button" onClick={() => setAddOpen(false)}
                className="px-3 py-1.5 rounded-lg text-ui-13 text-text-muted cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5 focus:text-text">Cancel</button>
              <button type="button" onClick={() => onAddServer(name, host, port, token)}
                className="px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer hover:bg-text/88 active:bg-text/50 focus:bg-text/88">Add</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
