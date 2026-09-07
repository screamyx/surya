// Rust: app/crates/ui/src/inbox/rules.rs (RulesPane::scope_line at line 123,
// RulesPane::render_rule at line 139).
// Render helpers on the same pane, not a second stateful component.
import type { AllowRule } from '../Rules';
import { bodyText, button, commandText, rowCard } from '../chrome';

// rules.rs:123. "Bash, in project-jag" / "Bash, everywhere" - where a rule
// reaches, said in words rather than as a path the user has to parse.
export function scopeLine(rule: AllowRule): string {
  if (rule.scope === 'global') return `${rule.toolName}, everywhere`;
  const trimmed = rule.workspacePath?.replace(/\/+$/, '');
  const folder = trimmed?.split('/').pop();
  return `${rule.toolName}, in ${folder ? folder : 'this workspace'}`;
}

// rules.rs:139.
export function renderRule(rule: AllowRule, onDeleteRule: (ruleId: string) => void) {
  return rowCard(`rule-${rule.id}`, (
    <>
      <div className="flex flex-row items-center gap-2">
        <div className="flex-1 min-w-0 truncate whitespace-nowrap text-ui-13 font-medium text-text">{rule.name}</div>
        {button('danger', 'Delete', {
          id: `delete-rule-${rule.id}`,
          ariaLabel: `Delete rule: ${rule.name}`,
          onClick: () => onDeleteRule(rule.id),
        })}
      </div>
      {bodyText(scopeLine(rule))}
      {commandText(rule.pattern)}
    </>
  ));
}
