// Rust: app/crates/ui/src/inbox/rules.rs (RulesPane, the Render impl at line 175).
import { renderRule } from './rules/rows';
import { emptyState } from './rules/chrome';

// Rust: app/crates/proto/src/state.rs:181, RuleScope. Serialized camelCase.
export type RuleScope = 'workspace' | 'global';

// Rust: app/crates/proto/src/state.rs:193, AllowRule. created_at is not read
// by any RulesPane render path, so it is not carried here.
export type AllowRule = {
  id: string;
  name: string;
  scope: RuleScope;
  workspacePath?: string;
  toolName: string;
  pattern: string;
  exact: boolean;
};

export type RulesProps = {
  rules: readonly AllowRule[];
  // rules.rs:21, RulesPane::failure. None in the quiet case.
  failure?: string;
  onDeleteRule: (ruleId: string) => void;
};

export function RulesPane({ rules, failure, onDeleteRule }: RulesProps) {
  return (
    <div id="rules-pane" className="size-full overflow-y-scroll flex flex-col gap-2 p-3">
      {failure && <div role="alert" className="text-ui-12 text-danger">{failure}</div>}
      {rules.length === 0 && emptyState('No always-allow rules. Every tool asks first.')}
      {rules.map(rule => renderRule(rule, onDeleteRule))}
    </div>
  );
}
