// Browser-only fixture bootstrap. Do not port this file into the native app.
import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { Shell } from './shell';
import type { Appearance } from './theme';
import type { Fixture } from './fixtures';
import { seededRows } from './fixtures';
import './tailwind.css';
const query = new URLSearchParams(window.location.search);
function Preview() {
  const [theme, setTheme] = useState<Appearance>(query.get('theme') === 'light' ? 'light' : 'dark');
  const [fixture, setFixture] = useState<Fixture>(query.get('state') === 'empty' ? 'empty' : 'seeded');
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [event, setEvent] = useState('');
  const rows = fixture === 'seeded' ? seededRows : [];
  function onEvent(action: string) {
    if (action === 'Toggle sidebar') setSidebarOpen(!sidebarOpen);
    else setEvent(`Preview event: ${action}`);
  }
  return <div data-theme={theme} className="h-full relative font-sans">
    <Shell rows={rows} fixture={fixture} sidebarOpen={sidebarOpen} onEvent={onEvent}
      onOpenChat={id => onEvent(`OpenChat(${id})`)} />
    {(query.get('proof') !== '1' || event) && <div className="absolute bottom-4 right-4 flex flex-col gap-2 p-3 rounded-lg border border-border bg-surface-overlay text-text text-ui-12">
      {query.get('proof') !== '1' && <div className="flex items-center gap-2">
        <span className="text-text-faint">Preview</span>
        <button type="button" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')} className="px-2 py-1 rounded-md border border-border-strong cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">{theme === 'dark' ? 'Use light' : 'Use dark'}</button>
        <button type="button" onClick={() => setFixture(fixture === 'seeded' ? 'empty' : 'seeded')} className="px-2 py-1 rounded-md border border-border-strong cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">{fixture === 'seeded' ? 'Show empty' : 'Show seeded'}</button>
      </div>}
      {event && <div role="status" className="flex items-center gap-2"><span>{event}</span><button type="button" aria-label="Dismiss preview event" onClick={() => setEvent('')} className="px-2 py-1 rounded-md cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">Dismiss</button></div>}
    </div>}
  </div>;
}
createRoot(document.getElementById('root')!).render(<Preview />);
