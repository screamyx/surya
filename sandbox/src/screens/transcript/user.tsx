// Rust: app/crates/ui/src/transcript.rs (RowKind::User's arm of render_row,
// render_user_attachments, user_bubble_text) and app/crates/ui/src/badges.rs
// (badges::render).

/// Rust: `crate::attachments::UserImageAttachment` plus the snapshot state
/// `Transcript::attachment_state` resolves per frame.
export type UserAttachment = {
  name: string;
  state: 'loaded' | 'loading' | 'error';
  /// The send is still crossing the relay: the thumbnail carries the wait.
  sending?: boolean;
  /// Transfer position, when a real transfer reports one. Absent means the
  /// indeterminate spinner, so the ring never shows a number that is not a
  /// real position.
  percent?: number;
};

/// Rust: `crate::badges::MessageBadge` - context the prompt folded in as text,
/// lifted back out above the bubble.
export type MessageBadge = { label: string; count?: number };

/// Rust: `crate::composer::SentMentionSpan` - a file-mention chip over the
/// display text, in display-byte terms. Carried here as the split text so the
/// browser needs no glyph geometry.
export type MentionSpan = { text: string; mention: boolean };

/// Rust: `render_user_attachments`. Thumbnails ride ABOVE the bubble, right
/// aligned; an image-only send shows no bubble at all.
export function userAttachments(atts: readonly UserAttachment[]) {
  return (
    <div className="w-full h-24 flex flex-row justify-end items-start gap-2 overflow-hidden px-1 pt-1">
      {atts.map(att => attachmentThumb(att))}
    </div>
  );
}

/// Rust: the three `AttachmentSnapshot` arms - a loaded sprite, the dashed
/// missing thumb, and the pulsing skeleton. The sandbox has no decoded image,
/// so a loaded thumb draws its frame and names the file.
function attachmentThumb(att: UserAttachment) {
  if (att.state === 'error') {
    return (
      <div key={att.name} data-attachment={att.name}
        className="flex-none w-28 h-20 rounded-lg overflow-hidden border border-wash/14 bg-wash/5" />
    );
  }
  if (att.state === 'loading') {
    return (
      <div key={att.name} data-attachment={att.name}
        className="flex-none w-28 h-20 rounded-lg overflow-hidden border border-wash/5 bg-wash/5 opacity-50" />
    );
  }
  return (
    <div key={att.name} data-attachment={att.name}
      className="flex-none w-28 h-20 rounded-lg overflow-hidden relative border border-wash/10 bg-wash/5 cursor-pointer flex items-end">
      <div className="w-full px-1.5 py-1 truncate whitespace-nowrap text-ui-10 text-text-muted">{att.name}</div>
      {att.sending === true && (
        <div className="absolute left-0 right-0 top-0 bottom-0 rounded-lg flex items-center justify-center bg-bg/50">
          <div data-upload-ring={att.name} className="size-8 rounded-full border border-text flex items-center justify-center text-ui-10 text-text">
            {att.percent === undefined ? '' : `${att.percent}`}
          </div>
        </div>
      )}
    </div>
  );
}

/// Rust: `crate::badges::render` - a 24px pill, icon plus label, over the
/// bubble and right-aligned with it.
export function userBadges(badges: readonly MessageBadge[]) {
  return (
    <div className="w-full flex flex-row flex-wrap justify-end items-center gap-1.5 pb-1.5">
      {badges.map(badge => (
        <div key={badge.label} data-badge={badge.label}
          className="h-6 flex flex-row items-center gap-1.5 px-2 rounded-lg bg-wash/5 text-ui-12 font-medium text-text-muted">
          {badge.label}
          {badge.count !== undefined && <span className="text-text-faint">{badge.count}</span>}
        </div>
      ))}
    </div>
  );
}

/// Rust: `user_bubble_text` - the sent text with its file-mention chips. Body
/// runs keep the sans face; chips read as inline code over the same rounded
/// wash the markdown renderer paints under inline code.
export function userBubbleText(spans: readonly MentionSpan[]) {
  return spans.map((span, ix) => (span.mention
    ? <span key={ix} className="rounded-sm px-0.5 bg-code-wash text-code-text">{span.text}</span>
    : <span key={ix}>{span.text}</span>));
}

/// Rust: the bubble itself. `min_w_0` is load-bearing natively because gpui
/// text answers min-content probes with its unwrapped width; the same class is
/// kept here so port-back reads one to one. A pending echo dims to 0.65.
export function userBubble(spans: readonly MentionSpan[], pending: boolean) {
  return (
    <div className="w-full flex justify-end">
      <div className={pending
        ? 'min-w-0 max-w-96 rounded-xl px-4 py-2.5 text-ui-14 text-text bg-wash/10 opacity-50'
        : 'min-w-0 max-w-96 rounded-xl px-4 py-2.5 text-ui-14 text-text bg-wash/10'}>
        {userBubbleText(spans)}
      </div>
    </div>
  );
}
