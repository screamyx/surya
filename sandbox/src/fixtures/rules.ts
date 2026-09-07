// Fixture props only. The native engine continues to own ListAllowRules and
// DeleteAllowRule. Mirrors app/crates/ui/src/inbox/mod.rs:249, demo::rules().
import type { AllowRule } from '../screens/Rules';

export const seededRules: readonly AllowRule[] = [
  {
    id: 'rule-1',
    name: 'Bash php artisan migrate* in orchard',
    scope: 'workspace',
    workspacePath: '/repos/orchard',
    toolName: 'Bash',
    pattern: 'php artisan migrate*',
    exact: false,
  },
  {
    id: 'rule-2',
    name: 'read-only git',
    scope: 'global',
    workspacePath: undefined,
    toolName: 'Bash',
    pattern: 'git status*',
    exact: false,
  },
];

// A workspace rule that somehow lost its path still reads sensibly rather than
// showing an empty gap (rules.rs:240, the_scope_line_names_the_folder_not_the_path).
export const pathlessRule: AllowRule = {
  id: 'rule-3',
  name: 'Edit anything in this project',
  scope: 'workspace',
  workspacePath: undefined,
  toolName: 'Edit',
  pattern: 'app/config/*.php',
  exact: false,
};

// rules.rs:82. What the pane shows when ListAllowRules does not answer.
export const rulesFailure = 'Could not load rules: engine is not running';

// The layout under pressure: rules.rs:151 truncates the name with
// .truncate().whitespace_nowrap() so the Delete button never gets pushed out.
// Not in seededRules; mount it when checking that boundary.
export const longNameRule: AllowRule = {
  id: 'rule-4',
  name: 'Bash docker compose exec -T app php vendor/bin/pest --filter PermissionCardTest in the orchard checkout on this device',
  scope: 'workspace',
  workspacePath: '/repos/orchard/',
  toolName: 'Bash',
  pattern: 'docker compose exec -T app php vendor/bin/pest --filter PermissionCardTest --stop-on-failure --colors=never',
  exact: true,
};
