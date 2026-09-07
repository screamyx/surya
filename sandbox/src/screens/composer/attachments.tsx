// Rust: app/crates/ui/src/composer.rs, Composer::render_attachment_strip (4002)
// and Composer::render_comments_chip (3974); app/crates/ui/src/attachments.rs,
// lightbox (801) and StagedAttachment (185).
import type { StagedAttachment } from '../../fixtures/composer';
import { renderIcon } from '../../icons';
import { messageBadge } from './chrome';

// comments.rs::chip_label - the count the badge names.
export function commentsChipLabel(count: number) {
  return count === 1 ? '1 comment' : `${count} comments`;
}

// render_comments_chip: px-4 pt-3 above the strip, the badges.rs pill.
export function renderCommentsChip(count: number) {
  if (count === 0) return null;
  return (
    <div className="flex flex-row px-4 pt-3">
      {messageBadge('chat-round-line', commentsChipLabel(count))}
    </div>
  );
}

// render_attachment_strip: `flex flex-wrap gap-2 px-4 pt-3`, 56px rounded
// thumbs, a remove button revealed on hover, click opens the full-size
// preview. The sandbox has no image asset, so each thumb paints its frame and
// names its extension where the photo would sit.
export function renderAttachmentStrip(
  staged: readonly StagedAttachment[],
  hovered: string | null,
  onHover: (id: string | null) => void,
  onPreview: (id: string) => void,
  onRemove: (id: string) => void,
) {
  if (staged.length === 0) return null;
  return (
    <div className="flex flex-row flex-wrap gap-2 px-4 pt-3">
      {staged.map(attachment => (
        <div key={attachment.id} className="relative flex"
          onMouseEnter={() => onHover(attachment.id)} onMouseLeave={() => onHover(null)}>
          <button type="button" data-attachment={attachment.id}
            aria-label={`Preview ${attachment.name}`} onClick={() => onPreview(attachment.id)}
            className="size-14 flex-none flex items-center justify-center rounded-lg overflow-hidden border border-wash/10 bg-wash/5 text-ui-10 text-text-faint cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
            {attachment.extension}
          </button>
          {hovered === attachment.id && (
            <button type="button" data-attachment-remove={attachment.id}
              aria-label={`Remove ${attachment.name}`} onClick={() => onRemove(attachment.id)}
              className="absolute top-0 right-0 size-5 flex-none flex items-center justify-center rounded-full bg-bg cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
              <span className="size-3.5 text-text-muted">{renderIcon('close-circle')}</span>
            </button>
          )}
        </div>
      ))}
    </div>
  );
}

// attachments.rs::lightbox - dim scrim, the image under a name, any click
// closes. The sandbox paints the frame the image would fill.
export function renderLightbox(attachment: StagedAttachment, onClose: () => void) {
  return (
    <button type="button" data-lightbox={attachment.id} aria-label="Close preview" onClick={onClose}
      className="absolute top-0 left-0 size-full flex flex-col items-center justify-center gap-3 bg-bg/88 cursor-pointer">
      <span className="w-96 h-64 rounded-md border border-wash/10 bg-wash/5 flex items-center justify-center text-ui-12 text-text-faint">
        {attachment.extension}
      </span>
      <span className="max-w-96 overflow-hidden text-ui-11 text-text-muted">{attachment.name}</span>
    </button>
  );
}
