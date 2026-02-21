---
name: illuc-fix-review
description: Address review feedback in an illuc task and resolve threads with clear updates.
metadata:
  short-description: Apply review feedback and resolve threads
---

# illuc-fix-review

Resolve code review comments created inside illuc’s diff review UI by:
1) reading the review threads stored in the worktree’s `.illuc/local-review.json`,
2) making the requested code changes (and tests where appropriate),
3) marking each addressed thread as `resolved` (or `wont-fix`/`closed`) and leaving a short “what changed” reply comment.

This skill is designed around how illuc actually stores review threads:
- Store file: `<worktreeRoot>/.illuc/local-review.json`
- One store contains `tasks[taskId].threads[]`
- A thread is identified by `(filePath, lineNumberOld, lineNumberNew)` and has:
  `status: active|pending|resolved|wont-fix|closed`
  `comments[]: { id, body, author, createdAt }`

## When To Use
- The user says “resolve review comments” or “address review feedback” and the feedback exists as illuc review threads.
- You want to keep an auditable trail in the review UI by replying and updating thread status.

## Workflow

### 1) Discover Open Threads
Run this from the task worktree (the repo you are editing):

```bash
ILLUC_REVIEW_HELPER="$HOME/.agents/skills/illuc-fix-review/illuc-review.py"
python3 "$ILLUC_REVIEW_HELPER" list
```

If task id inference fails (for example, you are not inside `.illuc/worktrees/<uuid>`), pass it explicitly:

```bash
python3 "$ILLUC_REVIEW_HELPER" list --task <task-uuid>
```

By default, `list` shows `active,pending`. To include everything:

```bash
python3 "$ILLUC_REVIEW_HELPER" list --status active,pending,resolved,wont-fix,closed
```

### 2) Address Each Thread (Code + Tests)
For each listed thread:
- Treat the comment text as the source of truth. The line numbers are anchors for the diff UI and may become stale after edits.
- Make the minimal, correct change that satisfies the comment.
- Follow repo conventions:
  - Angular: keep changes inside the owning feature slice under `src/app/features/...` (avoid cross-slice reach-in).
  - Tauri/Rust: keep changes inside the owning module under `src-tauri/src/features/...`.
- Add/adjust tests when the change is behaviorally meaningful.

Suggested checks (pick what’s relevant to your change):
- Frontend: `npm run build`
- Rust: `cargo test`

### 3) Reply + Mark As Resolved
Once you’ve implemented a thread’s request, mark it `resolved` and add a short reply describing what changed (include file paths / key decisions).

```bash
python3 "$ILLUC_REVIEW_HELPER" resolve --thread <n> --message "Fixed by: ... (files: ...)."
```

If you intentionally will not make the requested change, set `wont-fix` (and still leave a comment explaining why):

```bash
python3 "$ILLUC_REVIEW_HELPER" set-status --thread <n> --status wont-fix
python3 "$ILLUC_REVIEW_HELPER" comment --thread <n> --message "Won't fix: ... reasoning ..."
```

If the thread is obsolete due to a rewrite or file removal, use `closed` (and explain briefly):

```bash
python3 "$ILLUC_REVIEW_HELPER" set-status --thread <n> --status closed
python3 "$ILLUC_REVIEW_HELPER" comment --thread <n> --message "Closed: ... reason ..."
```

If you can’t fully address it in this pass, set `pending` and comment what remains:

```bash
python3 "$ILLUC_REVIEW_HELPER" set-status --thread <n> --status pending
python3 "$ILLUC_REVIEW_HELPER" comment --thread <n> --message "Pending: ... next steps ..."
```

## Definition Of Done
- No `active` threads remain for the target task unless explicitly deferred with `pending` and an explanation.
- Each resolved thread has a short reply comment summarizing the fix.
- Relevant checks pass (at least `npm run build` for TS changes and/or `cargo test` for Rust changes).

## Failure Modes / Escalation
- Multiple task ids in the store and task inference fails: ask the user for the correct task UUID or which review threads to target.
- A comment is ambiguous or conflicts with project direction: ask the user, do not guess and mark resolved.
