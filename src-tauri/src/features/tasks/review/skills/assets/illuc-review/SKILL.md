---
name: illuc-review
description: Review code changes in an illuc task and leave actionable feedback.
metadata:
  short-description: Review changes and leave actionable feedback threads
---

# illuc-review

Perform a code review in an illuc task worktree and write review comments into the worktree review store:
`<worktreeRoot>/.illuc/local-review.json`.

Comments must be authored as the reviewing agent (not `user`) so the UI renders the agent name directly.

## First Question To Ask
Ask the user which diff you should review:
1. Uncommitted changes in the worktree (like `git diff` + `git diff --staged`)
2. The whole branch compared to a base branch (like `git diff <base>...HEAD`)

If they choose option 2, also ask which base branch to compare against (for example `main`).

## How Illuc Stores Review Threads
- Store file: `<worktreeRoot>/.illuc/local-review.json`
- One store contains `tasks[taskId].threads[]`
- A thread is identified by `(filePath, lineNumberOld, lineNumberNew)` and has:
  `status: active|pending|resolved|wont-fix|closed`
  `comments[]: { id, body, author, createdAt }`

## Workflow

### 1) Pick The Review Target And Produce A Diff
For uncommitted changes:
- `git status -sb`
- `git diff`
- `git diff --staged`

For branch-vs-base:
- `git status -sb`
- `git diff <base>...HEAD`

Use the diff to enumerate changed files and focus review on:
- Correctness and edge cases
- Behavior regressions
- Error handling
- Security and data validation where relevant
- Consistency with the repo’s architecture and code style
- Missing tests

### 2) Anchor Each Comment To A File + Line
For each finding, pick an anchor in the *new* file content (preferred):
- Open the file and use real line numbers (for example `nl -ba <file> | sed -n '120,160p'`).
- Prefer `--line-new <N>` with `--line-type context` for most comments.
- Use `--line-old` only when the issue is about removed code.

### 3) Write Comments Into `.illuc/local-review.json` As The Agent
Set the author name once per session:
- Export `ILLUC_REVIEW_AUTHOR` (recommended), or
- Pass `--author` per command.

Examples:

```bash
export ILLUC_REVIEW_AUTHOR="codex"
ILLUC_REVIEW_HELPER="$HOME/.agents/skills/illuc-review/illuc-review.py"
python3 "$ILLUC_REVIEW_HELPER" add --file src/app/foo.ts --line-new 123 --line-type context --status active --message "Blocking: ... Suggested fix: ..."
python3 "$ILLUC_REVIEW_HELPER" add --file src/app/foo.ts --line-new 140 --status pending --message "Question: ...?"
python3 "$ILLUC_REVIEW_HELPER" add --file src/app/foo.ts --line-new 200 --status active --message "Nit: ... (optional)."
```

Notes:
- Keep each comment short and actionable.
- Include “why” and a concrete suggested change when possible.
- Use `pending` for questions/uncertainty, `active` for requested changes, and avoid `resolved` (the author should resolve threads).

### 4) Summarize The Review In Chat
After writing threads:
- List the highest-severity issues first.
- Call out any missing tests and what they should cover.
- Mention any architecture/slice boundary concerns explicitly.

## Definition Of Done
- Review comments exist in `.illuc/local-review.json` for the target task.
- Each comment is anchored to a plausible file + line number.
- Each comment’s `author` is the agent name (not `user`).
