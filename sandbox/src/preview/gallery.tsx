// Browser-only preview gallery. Do not port this file into the native app.
// It mounts each ported screen with its fixture props, the way src/main.tsx
// mounts the Needs you pane. Every screen file names its own Rust source.
import { useState } from 'react';
import type { ReactNode } from 'react';
import type { Fixture } from '../fixtures';
import { seededRows } from '../fixtures';
import { NeedsYouPane } from '../screens/NeedsYou';
import { InboxPane } from '../screens/Inbox';
import { RulesPane } from '../screens/Rules';
import { SpacesShell } from '../screens/Spaces';
import { TasksPane } from '../screens/Tasks';
import { FilesPane } from '../screens/Files';
import { TerminalPanel } from '../screens/Terminal';
import { BrowserPane } from '../screens/Browser';
import { Changes } from '../screens/Changes';
import { Pickers } from '../screens/Pickers';
import { GitHistory } from '../screens/History';
import { GitHistoryCount } from '../screens/history/count';
import { GitHistoryFetchButton } from '../screens/history/fetch_button';
import { Transcript } from '../screens/Transcript';
import { Composer } from '../screens/Composer';
import { SettingsWindow } from '../screens/Settings';
import { NotificationsPage } from '../screens/settings/notifications';
import { ArchivedPage } from '../screens/settings/archived';
import { ServersPage } from '../screens/settings/servers';
import { DevicesPage } from '../screens/settings/devices';
import { HarnessesPage } from '../screens/settings/harnesses';
import { ShortcutsPage } from '../screens/settings/shortcuts';
import { AccountsPage } from '../screens/settings/accounts';
import { AppearancePage } from '../screens/settings/appearance';
import { emptySections, seededSections } from '../fixtures/agents';
import { longNameRule, pathlessRule, seededRules } from '../fixtures/rules';
import { seededAgentRows, seededChats, seededSpaces, seededSurfaceTabs } from '../fixtures/spaces';
import { emptyTasks, seededTasks, tasksSpaceName } from '../fixtures/tasks';
import { expandedSeed, fileDocuments, fileEntries } from '../fixtures/files';
import { emptyTerminalTabs, seededTerminalTabs } from '../fixtures/terminal';
import { emptyPage, emptyTabs, seededPage, seededTabs, seededZoom } from '../fixtures/browser';
import { emptyDiff, seededDiff } from '../fixtures/changes';
import { emptyPickers, seededPickers } from '../fixtures/pickers';
import { historyBranch, historyCount, historyFixtures } from '../fixtures/history';
import { composerVariants } from '../fixtures/composer';
import { pendingPermission, sendingRows, workingNow,
  seededRows as seededTranscriptRows } from '../fixtures/transcript';
import {
  emptyNotifications, emptyStatus, localActiveRow, seededActiveRow, seededArchived,
  seededNotifications, seededServers, seededStatus, settingsSections,
} from '../fixtures/settings_frame';
import {
  defaultKeymap, localDeviceId, seededDevices, seededHarnesses, seededKeymap, shortcutCatalog,
} from '../fixtures/settings_devices';
import {
  emptyAccounts, emptyAppearance, seededAccounts, seededAppearance,
} from '../fixtures/settings_appearance';

export type ScreenId =
  | 'needs-you' | 'inbox' | 'rules' | 'spaces' | 'tasks' | 'files' | 'terminal'
  | 'browser' | 'changes' | 'pickers' | 'history' | 'transcript' | 'composer' | 'settings';

export const screenMenu: readonly { id: ScreenId; label: string }[] = [
  { id: 'needs-you', label: 'Needs you' },
  { id: 'inbox', label: 'Inbox' },
  { id: 'rules', label: 'Rules' },
  { id: 'spaces', label: 'Spaces' },
  { id: 'tasks', label: 'Tasks' },
  { id: 'files', label: 'Files' },
  { id: 'terminal', label: 'Terminal' },
  { id: 'browser', label: 'Browser' },
  { id: 'changes', label: 'Changes' },
  { id: 'pickers', label: 'Pickers' },
  { id: 'history', label: 'History' },
  { id: 'transcript', label: 'Transcript' },
  { id: 'composer', label: 'Composer' },
  { id: 'settings', label: 'Settings' },
];

export type GalleryProps = {
  screen: ScreenId;
  fixture: Fixture;
  onEvent: (action: string) => void;
};

function fill(child: ReactNode) {
  return <div className="size-full min-h-0">{child}</div>;
}

export function Gallery({ screen, fixture, onEvent }: GalleryProps) {
  const seeded = fixture === 'seeded';
  const [copiedSha, setCopiedSha] = useState('');
  const [section, setSection] = useState('notifications');
  if (screen === 'needs-you') {
    return fill(<NeedsYouPane rows={seeded ? seededRows : []} onOpenChat={id => onEvent(`OpenChat(${id})`)} />);
  }
  if (screen === 'inbox') {
    return fill(<InboxPane sections={seeded ? seededSections : emptySections} rows={seeded ? seededRows : []}
      selectedChatId={seeded ? 'chat-fold' : undefined}
      onSelectChat={id => onEvent(`SelectChat(${id})`)} onOpenChat={id => onEvent(`OpenChat(${id})`)} />);
  }
  if (screen === 'rules') {
    return fill(<RulesPane rules={seeded ? [...seededRules, pathlessRule, longNameRule] : []}
      onDeleteRule={id => onEvent(`DeleteRule(${id})`)} />);
  }
  if (screen === 'spaces') {
    return fill(<SpacesShell spaces={seeded ? seededSpaces : []} chats={seeded ? seededChats : []}
      agentRows={seeded ? seededAgentRows : []} tabs={seeded ? seededSurfaceTabs : []}
      target={seeded ? 'surya @ dtry' : null} harness={seeded ? 'claude' : 'none'}
      gitDetected={true} localDeviceId="dtry" jumpHints={false}
      onOpenChat={id => onEvent(`OpenChat(${id})`)}
      onSetArchived={(id, on) => onEvent(`SetArchived(${id}, ${String(on)})`)}
      onOpenChatMenu={id => onEvent(`ChatMenu(${id})`)} onAddSpace={() => onEvent('AddSpace')}
      onRenameSpace={(id, name) => onEvent(`RenameSpace(${id}, ${name})`)}
      onRemoveSpace={id => onEvent(`RemoveSpace(${id})`)} />);
  }
  if (screen === 'tasks') {
    return fill(<TasksPane spaceName={tasksSpaceName} tasks={seeded ? seededTasks : emptyTasks}
      onCreateTask={draft => onEvent(`CreateTask(${draft.title})`)}
      onUpdateTask={id => onEvent(`UpdateTask(${id})`)}
      onDeleteTask={id => onEvent(`DeleteTask(${id})`)} />);
  }
  if (screen === 'files') {
    return fill(<FilesPane entries={seeded ? fileEntries : []} expandedSeed={expandedSeed}
      documents={seeded ? fileDocuments : {}}
      onOpenFile={path => onEvent(`OpenFile(${path})`)} onSaveFile={path => onEvent(`SaveFile(${path})`)}
      onReloadFromDisk={path => onEvent(`ReloadFromDisk(${path})`)}
      onOverwrite={path => onEvent(`Overwrite(${path})`)} />);
  }
  if (screen === 'terminal') {
    return <div className="size-full flex flex-col justify-end bg-bg">
      <div className="h-80 flex-none">
        <TerminalPanel tabs={seeded ? seededTerminalTabs : emptyTerminalTabs} activeKey="term-1"
          selectedChat={seeded} draggedKey={null} onNewTab={() => onEvent('NewTab()')}
          onCloseTab={id => onEvent(`CloseTab(${id})`)} onToggleTerminal={() => onEvent('ToggleTerminal()')} />
      </div>
    </div>;
  }
  if (screen === 'browser') {
    return fill(<BrowserPane tabs={seeded ? seededTabs : emptyTabs} activeTabId="tab-1"
      page={seeded ? seededPage : emptyPage} offNote={null} findOpen={seeded}
      zoomPercent={seeded ? seededZoom : 100}
      onNavigate={url => onEvent(`Navigate(${url})`)} onOpenTab={() => onEvent('OpenTab()')}
      onCloseTab={id => onEvent(`CloseTab(${id})`)} onReloadOrStop={() => onEvent('ReloadOrStop()')}
      onBack={() => onEvent('Back()')} onForward={() => onEvent('Forward()')}
      onFind={(q, forward) => onEvent(`Find(${q}, ${String(forward)})`)}
      onZoom={percent => onEvent(`Zoom(${percent})`)} />);
  }
  if (screen === 'changes') {
    return fill(<Changes parsed={seeded ? seededDiff : emptyDiff} scope="workingTree" baseRef={null}
      error={null} scopedNotice={null} onSelectScope={scope => onEvent(`SelectScope(${scope})`)}
      onAddComment={(path, side, line) => onEvent(`AddComment(${path}, ${side}, ${line})`)} />);
  }
  if (screen === 'pickers') {
    return <div className="size-full flex flex-col justify-end bg-surface">
      <Pickers fixture={seeded ? seededPickers : emptyPickers}
        onPickModel={(harness, modelId) => onEvent(`PickModel(${harness}, ${modelId})`)}
        onPickRef={name => onEvent(`PickRef(${name})`)} onPickSpace={id => onEvent(`PickSpace(${id})`)}
        onPickDevice={id => onEvent(`PickDevice(${id})`)} onNewProject={() => onEvent('AddSpacePalette')}
        onSetAutoApprove={on => onEvent(`SetAutoApprove(${String(on)})`)}
        onRetry={kind => onEvent(`Retry(${kind})`)} />
    </div>;
  }
  if (screen === 'history') {
    const props = historyFixtures[seeded ? 'seeded' : 'empty'];
    return <div className="size-full flex flex-col bg-bg">
      <div className="h-9 flex-none flex flex-row items-center justify-between px-2 gap-2 border-b border-border">
        <GitHistoryCount count={historyCount} branch={historyBranch} />
        <GitHistoryFetchButton fetching={false} onFetchAll={() => onEvent('FetchAll()')} />
      </div>
      <div className="flex-1 min-h-0">
        <GitHistory hasRepository={props.hasRepository} commits={props.commits} graph={props.graph}
          hasMore={props.hasMore} loading={props.loading} error={props.error} fetchError={props.fetchError}
          copiedSha={copiedSha} refAreaWidth={props.refAreaWidth}
          onOpenCommit={sha => onEvent(`OpenCommit(${sha})`)} onCopySha={sha => setCopiedSha(sha)}
          onLoadOlder={() => onEvent('LoadOlder()')} />
      </div>
    </div>;
  }
  if (screen === 'transcript') {
    return fill(<Transcript rows={seeded ? seededTranscriptRows : sendingRows}
      working={seeded ? workingNow : null} undelivered={!seeded}
      permission={seeded ? pendingPermission : null}
      onOpenSubagent={(docId, title) => onEvent(`OpenSubagent(${docId}, ${title})`)}
      onCopyMessage={id => onEvent(`CopyMessage(${id})`)} onCopyCode={() => onEvent('CopyCode')}
      onRetryTurn={prompt => onEvent(`RetryTurn(${prompt})`)} onRetrySend={() => onEvent('RetrySend')}
      onAnswerPermission={(id, answer) => onEvent(`AnswerPermission(${id}, ${answer})`)} />);
  }
  if (screen === 'composer') {
    return <div className="size-full flex flex-col justify-end bg-surface">
      <Composer model={seeded ? composerVariants.seeded : composerVariants.empty}
        onEvent={action => onEvent(action)} />
    </div>;
  }
  return fill(<SettingsWindow sections={settingsSections} section={section}
    renderSection={id => renderSettingsSection(id, seeded, onEvent)}
    onOpenSection={id => { setSection(id); onEvent(`OpenSection(${id})`); }}
    onBack={() => onEvent('CloseSettings')} />);
}

function renderSettingsSection(id: string, seeded: boolean, onEvent: (action: string) => void) {
  if (id === 'notifications') {
    return <NotificationsPage flags={seeded ? seededNotifications : emptyNotifications}
      onChanged={f => onEvent(`Notifications(${String(f.sound)},${String(f.desktop)})`)} />;
  }
  if (id === 'archived') {
    return <ArchivedPage rows={seeded ? seededArchived : []} busyId={null} error={null}
      onUnarchive={rowId => onEvent(`Unarchive(${rowId})`)} onDismissError={() => onEvent('DismissError')} />;
  }
  if (id === 'servers') {
    return <ServersPage servers={seeded ? seededServers : []} active={seeded ? seededActiveRow : localActiveRow}
      status={seeded ? seededStatus : emptyStatus} addError={null}
      onConnect={rowId => onEvent(`Connect(${rowId})`)} onRemove={rowId => onEvent(`Remove(${rowId})`)}
      onSaveCommandLine={() => onEvent('SaveCommandLine')}
      onAddServer={(n, h) => onEvent(`AddServer(${n},${h})`)} />;
  }
  if (id === 'devices') {
    return <DevicesPage devices={seeded ? seededDevices : []} localDeviceId={localDeviceId}
      workspaceScope="synced" onRename={(rowId, name) => onEvent(`RenameDevice(${rowId}, ${name})`)}
      onCopyId={rowId => onEvent(`CopyDeviceId(${rowId})`)} />;
  }
  if (id === 'harnesses') {
    return <HarnessesPage harnesses={seeded ? seededHarnesses : []} state="ready" devices={seededDevices}
      localDeviceId={localDeviceId} yoloDefault={false}
      onToggle={(rowId, on) => onEvent(`SetHarnessEnabled(${rowId}, ${String(on)})`)}
      onTargetDevice={rowId => onEvent(`TargetDevice(${rowId})`)} onRetry={() => onEvent('ListHarnesses')}
      onYoloDefault={on => onEvent(`YoloDefault(${String(on)})`)} />;
  }
  if (id === 'agents') {
    const a = seeded ? seededAccounts : emptyAccounts;
    return <AccountsPage status={a.status} accounts={a.accounts} warnings={a.warnings}
      devices={a.devices} localDeviceId={a.localDeviceId} busyAccountId={a.busyAccountId}
      actionError={a.actionError} loadError={a.loadError} login={a.login}
      onRefresh={() => onEvent('RefreshAccounts')}
      onAddAccount={harness => onEvent(`AddAccount(${harness})`)}
      onSwitchAccount={id2 => onEvent(`SwitchAccount(${id2})`)}
      onForgetAccount={id2 => onEvent(`ForgetAccount(${id2})`)} />;
  }
  if (id === 'appearance') {
    const v = seeded ? seededAppearance : emptyAppearance;
    return <AppearancePage mode={v.mode} themes={v.themes} lightVariants={v.lightVariants}
      darkVariants={v.darkVariants} accent={v.accent} surface={v.surface}
      resolvedSurface={v.resolvedSurface} motion={v.motion} systemReducedMotion={v.systemReducedMotion}
      font={v.font} fontChoices={v.fontChoices} fontSize={v.fontSize} fontSizes={v.fontSizes}
      fontFailed={v.fontFailed} library={v.library} libraryWarning={v.libraryWarning}
      onSetMode={mode => onEvent(`SetMode(${mode})`)}
      onSetTheme={(kind, variantId) => onEvent(`SetTheme(${kind}, ${variantId})`)}
      onSetAccent={accent => onEvent(`SetAccent(${String(accent)})`)}
      onSetMotion={motion => onEvent(`SetMotion(${motion})`)}
      onSetFont={font => onEvent(`SetFont(${font})`)}
      onSetFontSize={size => onEvent(`SetFontSize(${size})`)} />;
  }
  if (id === 'shortcuts') {
    return <ShortcutsPage catalog={shortcutCatalog} keymap={seeded ? seededKeymap : defaultKeymap}
      onChanged={next => onEvent(`KeymapChanged(${Object.keys(next).length})`)} />;
  }
  return <div className="flex flex-col items-center justify-center py-10 text-ui-13 text-text-faint">
    This page is not ported yet.
  </div>;
}
