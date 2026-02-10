#!/bin/bash
# Hook: Inject workflow reminders
# Matcher: UserPromptSubmit (runs on every user prompt)
#
# CUSTOMIZATION:
# - Update the reminder message to reflect your team's workflow
# - Add/remove data sources based on your setup
# - Adjust the frequency by changing when this hook runs
# - Set continueOnError: false in settings.json if you want to ensure this runs

set -euo pipefail

# CUSTOMIZATION: Update this message to reflect your data workflow and tools
# Output a brief reminder
echo "💡 Workflow: Linear=source of truth | claude-mem=cross-session memory | context7=verify APIs | plan files=planner only"

exit 0
