# Save Task List for Reuse

Save the current session's task list so it can be reused in future sessions.

## Instructions

1. **Find the current task list ID** by looking at the scratchpad path or checking `~/.claude/tasks/` for the most recently modified directory.

2. **Make sure the recently modified directory matches** the most recently modified directory to find the task list, then make sur you use the corect task list by comparing to your on going or completed tasks.

3. **Copy and store the task list ID** the id will be used to restore the task list in future sessions.

4. **Output the startup command** for the user:

   ```
   To continue with this task list in a new session:

   CLAUDE_CODE_TASK_LIST_ID=<task_list_id> claude
   ```

5. **Show the current task summary** so the user knows what's preserved.

## Example Usage

```
/save-tasks
```

This will:

- Output: `CLAUDE_CODE_TASK_LIST_ID=<task_list_id> claude`
