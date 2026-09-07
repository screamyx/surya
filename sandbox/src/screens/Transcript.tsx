// Rust: app/crates/ui/src/transcript.rs (Transcript, its Render impl and the
// RowKind model rows_for_entry builds).
import { useState } from 'react';
import { renderRow } from './transcript/rows';
import { PermissionCard } from './transcript/permission_card';
import type { PermissionAnswer, PermissionAsk } from './transcript/permission_card';
import type { CodeState } from './transcript/code_block';
import type { MdBlock } from './transcript/markdown';
import type { ToolItem } from './transcript/chip_header';
import type { ToolGroupState as GroupState } from './transcript/tool_group';
import type { MentionSpan, MessageBadge, UserAttachment } from './transcript/user';
import type { WorkingTrailer } from './transcript/working';

/// Rust: `RowKind`. One row is one top-level markdown block, one tool group,
/// one chip, or one user message.
export type RowKind =
  | {
      type: 'user';
      spans: readonly MentionSpan[];
      attachments: readonly UserAttachment[];
      badges: readonly MessageBadge[];
      pending: boolean;
    }
  | { type: 'markdown'; block: MdBlock }
  | { type: 'liveMarkdown'; block: MdBlock }
  | { type: 'toolGroup'; tools: readonly ToolItem[]; autoOpen: boolean; summary: string }
  | { type: 'inputChip'; header: string; resolved: boolean }
  | {
      type: 'permissionChip';
      toolName: string;
      command: string;
      resolved: boolean;
      allowed: boolean;
      always: boolean;
    }
  | { type: 'errorChip'; message: string };

/// Rust: `Row` - stable id, the owning message entry, and the marks that ride
/// the LAST row of a turn. `timestamp` is pre-formatted by `format_timestamp`
/// against the host's timezone, which stays in Rust.
export type TranscriptRow = {
  id: string;
  entryId: string;
  /// Rust: `part_prefix(&row.id)` - two markdown rows of the same part sit at
  /// the tighter block gap.
  partKey: string;
  turnStart: boolean;
  timestamp?: string;
  copyText?: string;
  stopped: boolean;
  retryPrompt?: string;
  kind: RowKind;
};

/// Everything `render_row` reads off `&mut self`: the view state the pane owns
/// and the intents that leave it. Bundled so the row helpers keep the same
/// shape as their Rust counterparts instead of taking a dozen arguments.
export type TranscriptUi = {
  hoveredEntry: string | null;
  copiedEntry: string | null;
  working: WorkingTrailer | null;
  undelivered: boolean;
  code: CodeState;
  toolGroup: GroupState;
  onHoverEntry: (rowId: string, entryId: string | null) => void;
  onCopyMessage: (entryId: string, text: string) => void;
  onRetryTurn: (prompt: string) => void;
  onRetrySend: () => void;
};

export type TranscriptProps = {
  rows: readonly TranscriptRow[];
  working?: WorkingTrailer | null;
  undelivered?: boolean;
  /// Natively the permission card is hosted by the composer, in the
  /// composer's place. It is rendered under the list here so the card and the
  /// row it answers can be looked at together.
  permission?: PermissionAsk | null;
  onOpenSubagent: (docId: string, title: string) => void;
  onCopyMessage: (entryId: string, text: string) => void;
  onCopyCode: (code: string) => void;
  onRetryTurn: (prompt: string) => void;
  onRetrySend: () => void;
  onAnswerPermission: (requestId: string, answer: PermissionAnswer) => void;
};

/// Rust: `impl Render for Transcript`. The list fills the pane and scrolls
/// under the chrome; the scroll-to-bottom pill, the edge fade, the selection
/// registry and the rail are the shell's or `rail.rs`'s, not this file's.
export function Transcript(props: TranscriptProps) {
  const { rows, permission } = props;
  const [hoveredEntry, setHoveredEntry] = useState<string | null>(null);
  const [hoveredRow, setHoveredRow] = useState<string | null>(null);
  const [copiedEntry, setCopiedEntry] = useState<string | null>(null);
  const [folds, setFolds] = useState<Record<string, boolean>>({});
  const [details, setDetails] = useState<Record<string, boolean>>({});
  const [fits, setFits] = useState<Record<string, boolean>>({});
  const [codeHover, setCodeHover] = useState<Record<string, boolean>>({});
  const [copiedCode, setCopiedCode] = useState<string | null>(null);
  const [picked, setPicked] = useState<PermissionAnswer | null>(null);

  const code: CodeState = {
    fit: key => fits[key] === true,
    hovered: key => codeHover[key] === true,
    copied: key => copiedCode === key,
    onToggleFit: key => setFits({ ...fits, [key]: fits[key] !== true }),
    onHoverCode: (key, on) => setCodeHover({ ...codeHover, [key]: on }),
    onCopyCode: (key, text) => { setCopiedCode(key); props.onCopyCode(text); },
  };
  const toolGroup: GroupState = {
    open: (rowId, autoOpen) => folds[rowId] ?? autoOpen,
    detailOpen: (key, defaultOpen) => details[key] ?? defaultOpen,
    onToggleGroup: (rowId, autoOpen) => setFolds({ ...folds, [rowId]: !(folds[rowId] ?? autoOpen) }),
    onToggleDetail: (key, defaultOpen) => setDetails({ ...details, [key]: !(details[key] ?? defaultOpen) }),
    onOpenSubagent: props.onOpenSubagent,
  };
  const ui: TranscriptUi = {
    hoveredEntry,
    copiedEntry,
    working: props.working ?? null,
    undelivered: props.undelivered === true,
    code,
    toolGroup,
    // Only the row that OWNS the current reveal may clear it: a stale leave
    // from an earlier row must not blank the strip a newly entered row lit.
    onHoverEntry: (rowId, entryId) => {
      if (entryId !== null) { setHoveredRow(rowId); setHoveredEntry(entryId); }
      else if (hoveredRow === rowId) { setHoveredRow(null); setHoveredEntry(null); }
    },
    onCopyMessage: (entryId, text) => { setCopiedEntry(entryId); props.onCopyMessage(entryId, text); },
    onRetryTurn: props.onRetryTurn,
    onRetrySend: props.onRetrySend,
  };

  return (
    <section aria-label="Chat transcript" className="relative size-full min-h-0 flex flex-col">
      <div className="w-full flex-1 min-h-0 overflow-y-scroll flex flex-col">
        {rows.map((_, ix) => renderRow(rows, ix, ui))}
      </div>
      {permission !== null && permission !== undefined && (
        <div className="w-full flex-none flex justify-center px-12 pb-4">
          <div className="w-full max-w-184 min-w-0">
            <PermissionCard ask={permission} picked={picked}
              onAnswerPermission={(requestId, answer) => {
                setPicked(answer);
                props.onAnswerPermission(requestId, answer);
              }} />
          </div>
        </div>
      )}
    </section>
  );
}
