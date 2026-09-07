// Rust: shell.rs render_sidebar / render_rail_entries / render_chat_sidebar.
// The transcript minimap in rail.rs is not present on the Needs you page.
import type { Fixture } from '../fixtures';
import { emptyChats } from '../fixtures';
import { badge } from '../screens/chrome';
import { renderIcon } from './icons';
import { iconButton } from './titlebar';
export function renderSidebar(waiting: number, fixture: Fixture, onEvent: (value: string) => void) {
  const chats = fixture === 'seeded' ? [
    { title: 'Wire the Tasks pane into the shell', project: 'surya @ WINBOX', age: 'Input', branch: '' },
  ] : emptyChats;
  return <aside aria-label="Sidebar" className="w-64 flex-none h-full flex flex-col pt-9.5 bg-wash/5 border-r border-border">
    <nav aria-label="Main navigation" className="flex flex-col gap-0.5 px-2 pt-1.5 pb-2 border-b border-border">
      {(['Home', 'Needs you', 'Agents', 'Tasks', 'Files'] as const).map((label, i) => (
        <button key={label} type="button" onClick={() => onEvent(label)} aria-current={label === 'Needs you' ? 'page' : undefined}
          className={label === 'Needs you'
            ? 'flex items-center gap-2.5 h-7.5 px-2.5 rounded-lg text-ui-13 text-text bg-element-active cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
            : 'flex items-center gap-2.5 h-7.5 px-2.5 rounded-lg text-ui-13 text-text cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
          <span className="size-3.75 flex-none text-text-muted">{renderIcon((['home', 'bell', 'bot', 'checklist', 'folder'] as const)[i])}</span>
          {label}
          {label === 'Needs you' && waiting > 0 && <><span className="flex-1" />{badge(String(waiting), 'accent')}</>}
        </button>
      ))}
    </nav>
    <div className="flex-none flex flex-col px-2 pt-2.5 pb-2 border-b border-border">
      <span className="px-1 pt-2.5 pb-1 text-ui-10 font-medium text-text-faint">{waiting ? 'Waiting for you' : 'Done'}</span>
      <button type="button" onClick={() => onEvent('Open agent')} className="flex items-center gap-2 h-7.5 px-1 rounded-md text-ui-13 text-text-muted text-left cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
        <span className={waiting ? 'size-1.5 flex-none rounded-full bg-warning' : 'size-1.5 flex-none rounded-full bg-success'} />
        <span className="truncate">{chats[0].title}</span>
      </button>
    </div>
    <div className="flex-1 min-h-0 flex flex-col px-2">
      <div className="h-11 flex items-center gap-2 px-1.5">
        <button type="button" onClick={() => onEvent('All projects')} className="flex-1 min-w-0 flex items-center gap-2 text-ui-13 text-text-muted rounded-md cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
          <span className="size-4">{renderIcon('folder')}</span>All projects
          <span className="flex-1" /><span className="size-3">{renderIcon('alt-arrow-down')}</span>
        </button>
        {iconButton('sort', 'Filter chats', onEvent)}
      </div>
      {chats.map((chat, index) => (
        <button key={chat.title} type="button" onClick={() => onEvent(`Open chat: ${chat.title}`)}
          className={index === 0
            ? 'flex flex-col min-w-0 px-2 py-1 rounded-lg text-left bg-element-active cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
            : 'flex flex-col min-w-0 px-2 py-1 rounded-lg text-left cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
          <span className="w-full flex items-center justify-between text-ui-10 text-text-faint"><span>{chat.project}</span><span>{chat.age}</span></span>
          <span className="w-full flex items-center gap-2 text-ui-13 text-text"><span className="size-3 flex-none text-claude-brand">{renderIcon('claude-mark')}</span><span className="truncate">{chat.title}</span></span>
          {chat.branch && <span className="w-full flex items-center gap-1 text-ui-10 text-text-faint"><span className="size-3">{renderIcon('git-branch')}</span>{chat.branch}<span className="flex-1" /><span className="text-success bg-success/14 rounded-sm px-0.5">#266</span></span>}
        </button>
      ))}
    </div>
    <button type="button" aria-label="Local only" onClick={() => onEvent('Local account')} className="flex-none flex items-center gap-2.5 h-16 px-4 text-ui-13 font-medium cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
      <span className="size-7 flex items-center justify-center rounded-full bg-solid text-on-solid">L</span>Local only
    </button>
  </aside>;
}
