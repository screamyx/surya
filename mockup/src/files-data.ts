import type { FileNode } from "./data"

export const fileTree: FileNode[] = [
  {
    name: "backend", path: "backend", kind: "dir", children: [
      { name: "app", path: "backend/app", kind: "dir", children: [
        { name: "Models", path: "backend/app/Models", kind: "dir", children: [
          { name: "Lead.php", path: "backend/app/Models/Lead.php", kind: "file", touched: "agent" },
          { name: "Vehicle.php", path: "backend/app/Models/Vehicle.php", kind: "file" },
        ]},
        { name: "Jobs", path: "backend/app/Jobs", kind: "dir", children: [
          { name: "SendFollowUpReminder.php", path: "backend/app/Jobs/SendFollowUpReminder.php", kind: "file", touched: "agent" },
        ]},
      ]},
      { name: "database", path: "backend/database", kind: "dir", children: [
        { name: "migrations", path: "backend/database/migrations", kind: "dir", children: [
          { name: "2026_09_05_000001_add_follow_up_to_leads.php", path: "backend/database/migrations/2026_09_05_000001_add_follow_up_to_leads.php", kind: "file", touched: "agent" },
        ]},
      ]},
      { name: "tests", path: "backend/tests", kind: "dir", children: [
        { name: "Feature", path: "backend/tests/Feature", kind: "dir", children: [
          { name: "FollowUpReminderTest.php", path: "backend/tests/Feature/FollowUpReminderTest.php", kind: "file", touched: "agent" },
        ]},
      ]},
      { name: "staff-react", path: "backend/staff-react", kind: "dir", children: [
        { name: "src", path: "backend/staff-react/src", kind: "dir", children: [
          { name: "LeadCard.tsx", path: "backend/staff-react/src/LeadCard.tsx", kind: "file", touched: "you" },
        ]},
      ]},
    ],
  },
  { name: "specs", path: "specs", kind: "dir", children: [
    { name: "behaviors", path: "specs/behaviors", kind: "dir", children: [
      { name: "crm-lead-follow-up.md", path: "specs/behaviors/crm-lead-follow-up.md", kind: "file" },
    ]},
  ]},
  { name: "AGENTS.md", path: "AGENTS.md", kind: "file" },
]

export const fileContents: Record<string, string> = {
  "backend/app/Models/Lead.php": `<?php

namespace App\\Models;

use Illuminate\\Database\\Eloquent\\Model;

class Lead extends Model
{
    protected $fillable = [
        'customer_id',
        'vehicle_id',
        'source',
        'next_follow_up_at',
        'follow_up_note',
    ];

    protected function casts(): array
    {
        return [
            'next_follow_up_at' => 'datetime',
        ];
    }

    public function isOverdue(): bool
    {
        return $this->next_follow_up_at?->isPast() ?? false;
    }
}
`,
  "backend/app/Jobs/SendFollowUpReminder.php": `<?php

namespace App\\Jobs;

use App\\Models\\Lead;
use Illuminate\\Contracts\\Queue\\ShouldQueue;
use Illuminate\\Foundation\\Queue\\Queueable;

class SendFollowUpReminder implements ShouldQueue
{
    use Queueable;

    public function __construct(public Lead $lead) {}

    public function handle(): void
    {
        if ($this->lead->next_follow_up_at === null) {
            return;
        }

        // TODO: push to the salesperson's phone
    }
}
`,
  "specs/behaviors/crm-lead-follow-up.md": `# Behavior: lead follow-up

Every lead carries a next follow-up date and a one-line note.
The morning after a lead comes in, the salesperson gets one reminder.
`,
}

export const leadPhpBefore = `    protected $fillable = [
        'customer_id',
        'vehicle_id',
        'source',
    ];
`
export const leadPhpAfter = `    protected $fillable = [
        'customer_id',
        'vehicle_id',
        'source',
        'next_follow_up_at',
        'follow_up_note',
    ];

    protected function casts(): array
    {
        return [
            'next_follow_up_at' => 'datetime',
        ];
    }
`

