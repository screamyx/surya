// Rust: app/crates/ui/src/settings/notifications.rs (NotificationsPage::render).
// The page holds a working copy of the three flags and reports every flip; the
// shell persists them (NotificationsEvent::Changed).
import { useState } from 'react';
import type { ReactNode } from 'react';
import { pageColumn, pageHeader, pageSubtitle, sectionCard, cardRow, rowTile, rowTitle, metaLine, toggleSwitch } from './widgets';

export type NotificationFlags = { sound: boolean; desktop: boolean; backgroundOnly: boolean };
export type NotificationsProps = {
  flags: NotificationFlags;
  onChanged: (flags: NotificationFlags) => void;
};

function row(first: boolean, icon: 'volume-loud' | 'bell' | 'monitor', title: string, copy: string,
  toggle: ReactNode, dimmed = false) {
  return cardRow(first, (
    <>
      {rowTile(icon)}
      <div className="flex-1 min-w-0 flex flex-col">
        {rowTitle(title)}
        {metaLine([copy])}
      </div>
      {toggle}
    </>
  ), dimmed);
}

export function NotificationsPage({ flags, onChanged }: NotificationsProps) {
  const [sound, setSound] = useState(flags.sound);
  const [desktop, setDesktop] = useState(flags.desktop);
  const [backgroundOnly, setBackgroundOnly] = useState(flags.backgroundOnly);
  function change(next: NotificationFlags) {
    setSound(next.sound);
    setDesktop(next.desktop);
    setBackgroundOnly(next.backgroundOnly);
    onChanged(next);
  }
  return (
    <div className="size-full overflow-y-scroll" data-page="notifications">
      {pageColumn(
        <>
          {pageHeader('Notifications')}
          {pageSubtitle('How session pings reach you when a run finishes or an agent is waiting on your input.', true)}
          {sectionCard(
            <>
              {row(true, 'volume-loud', 'Sounds',
                'Chime when a run finishes or an agent asks a question.',
                toggleSwitch(sound, 'Sounds', () => change({ sound: !sound, desktop, backgroundOnly })))}
              {row(false, 'bell', 'Desktop notifications',
                'Show a system banner on the same events, so pings reach you while Surya is in the background.',
                toggleSwitch(desktop, 'Desktop notifications', () => change({ sound, desktop: !desktop, backgroundOnly })))}
              {row(false, 'monitor', 'Only when in the background',
                'Skip the banner while a Surya window is focused - the chime already covers it.',
                toggleSwitch(backgroundOnly, 'Only when in the background',
                  () => change({ sound, desktop, backgroundOnly: !backgroundOnly }), !desktop), !desktop)}
            </>
          )}
        </>
      )}
    </div>
  );
}
