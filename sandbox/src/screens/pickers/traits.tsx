// Rust: app/crates/ui/src/pickers.rs - render_traits_sections (3526) and
// default_badge (3634). The reasoning ladder plus every advertised model option
// as headed sections of menu rows, split by hairline separators. Selecting
// keeps the menu open for multi-adjust.
import type { ReactNode } from 'react';
import { menuHeading, menuSeparator, skeletonMenuRows, traitRow } from './popover';
import type { PickerModel, ReasoningLevel } from '../../fixtures/pickers';

/// default_reasoning: High when the ladder offers it, else Medium, else first.
export function defaultReasoning(ladder: readonly ReasoningLevel[]): ReasoningLevel | null {
  if (ladder.includes('High')) return 'High';
  if (ladder.includes('Medium')) return 'Medium';
  return ladder[0] ?? null;
}

/// The "Default" marker beside a section's default choice: a ghost badge, bare
/// muted text with no border or fill.
function defaultBadge() {
  return <span className="flex-none text-ui-10 font-semibold text-text-faint">Default</span>;
}

export type TraitsProps = {
  model: PickerModel | null;
  reasoning: ReasoningLevel | null;
  options: Readonly<Record<string, string>>;
  onPickReasoning: (level: ReasoningLevel) => void;
  onPickOption: (optionId: string, choiceId: string) => void;
};

export function renderTraitsSections(props: TraitsProps) {
  if (!props.model) return skeletonMenuRows(3);
  const model = props.model;
  const fallback = defaultReasoning(model.ladder);
  const sections: ReactNode[] = [];
  if (model.ladder.length > 0) {
    sections.push(
      <div key="reasoning" className="flex flex-col gap-0.5">
        {menuHeading('Reasoning')}
        {model.ladder.map((level, ix) => traitRow(
          `trait-reasoning-${ix}`, props.reasoning === level, () => props.onPickReasoning(level),
          <>
            <span>{level}</span>
            <span className="flex-1" />
            {fallback === level && defaultBadge()}
          </>,
        ))}
      </div>,
    );
  }
  model.options.forEach((option, optIx) => {
    if (sections.length > 0) sections.push(menuSeparator(`trait-sep-${optIx}`));
    const picked = props.options[option.id] ?? option.defaultChoice;
    sections.push(
      <div key={option.id} className="flex flex-col gap-0.5">
        {menuHeading(option.label)}
        {option.choices.map((choice, choiceIx) => traitRow(
          `trait-choice-${optIx}-${choiceIx}`, picked === choice.id,
          () => props.onPickOption(option.id, choice.id),
          <>
            <span>{choice.label}</span>
            <span className="flex-1" />
            {choice.id === option.defaultChoice && defaultBadge()}
          </>,
        ))}
      </div>,
    );
  });
  return <div className="flex flex-col pb-0.5">{sections}</div>;
}
