#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import subprocess
import sys
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Optional


UUID_RE = re.compile(
    r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$",
    re.IGNORECASE,
)


def iso_utc_now() -> str:
    # Match typical RFC3339 serialization used by serde/chrono (e.g. "...Z").
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def try_git_toplevel() -> Optional[Path]:
    try:
        out = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            stderr=subprocess.DEVNULL,
            text=True,
        ).strip()
        return Path(out) if out else None
    except Exception:
        return None


def is_uuid(value: str) -> bool:
    return bool(UUID_RE.match(value.strip()))


def read_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as f:
        return json.load(f)


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, indent=2, ensure_ascii=True) + "\n"
    path.write_text(payload, encoding="utf-8")


def ensure_store(raw: Any) -> dict[str, Any]:
    if not isinstance(raw, dict):
        return {"version": 1, "tasks": {}}
    tasks = raw.get("tasks")
    return {
        "version": raw.get("version", 1) if isinstance(raw.get("version"), int) else 1,
        "tasks": tasks if isinstance(tasks, dict) else {},
    }


def infer_task_id(explicit: Optional[str], worktree_root: Path, store: dict[str, Any]) -> Optional[str]:
    if explicit:
        return explicit
    base = worktree_root.name
    if is_uuid(base):
        return base
    keys = list((store.get("tasks") or {}).keys())
    if len(keys) == 0:
        return None
    if len(keys) == 1:
        return keys[0]
    raise RuntimeError(
        f"Unable to infer task id. Pass --task <uuid>. Found {len(keys)} tasks in store."
    )


@dataclass
class Thread:
    file_path: str
    line_number_old: Optional[int]
    line_number_new: Optional[int]
    line_type: str
    status: str
    comments: list[dict[str, Any]]


def normalize_thread(raw: Any) -> Thread:
    if not isinstance(raw, dict):
        raise RuntimeError("Invalid thread shape in store")
    file_path = raw.get("filePath", raw.get("file_path"))
    if not isinstance(file_path, str) or not file_path.strip():
        raise RuntimeError("Invalid thread filePath")
    lno = raw.get("lineNumberOld", raw.get("line_number_old"))
    lnn = raw.get("lineNumberNew", raw.get("line_number_new"))
    line_number_old = int(lno) if isinstance(lno, int) else None
    line_number_new = int(lnn) if isinstance(lnn, int) else None
    line_type = raw.get("lineType", raw.get("line_type")) or "context"
    status = raw.get("status") or "active"
    comments = raw.get("comments")
    if not isinstance(comments, list):
        comments = []
    return Thread(
        file_path=file_path,
        line_number_old=line_number_old,
        line_number_new=line_number_new,
        line_type=str(line_type),
        status=str(status),
        comments=comments,
    )


def thread_key(t: Thread) -> str:
    old_part = "x" if t.line_number_old is None else str(t.line_number_old)
    new_part = "x" if t.line_number_new is None else str(t.line_number_new)
    return f"{t.file_path}::{old_part}::{new_part}"


def list_threads(entry: Any) -> list[Thread]:
    threads_raw = entry.get("threads") if isinstance(entry, dict) else None
    if not isinstance(threads_raw, list):
        return []
    threads = [normalize_thread(t) for t in threads_raw]
    threads.sort(key=lambda t: (t.file_path, t.line_number_new or 0))
    return threads


def latest_comment_preview(t: Thread) -> str:
    if not t.comments:
        return ""
    last = t.comments[-1]
    body = last.get("body") if isinstance(last, dict) else None
    if not isinstance(body, str) or not body.strip():
        return ""
    first = body.splitlines()[0][:120]
    return first.replace('"', "'")


def cmd_list(store_path: Path, store: dict[str, Any], task_id: Optional[str], statuses: list[str]) -> int:
    if not task_id:
        if not store_path.exists():
            print(f"No review store found at {store_path}")
            return 0
        print(f"No review tasks found in {store_path}")
        return 0

    entry = (store.get("tasks") or {}).get(task_id)
    if not isinstance(entry, dict):
        print(f"No review entry for task {task_id} in {store_path}")
        return 0

    all_threads = list_threads(entry)
    if statuses:
        visible = [t for t in all_threads if t.status in statuses]
    else:
        visible = all_threads

    if not visible:
        print(f"No threads matching status=[{', '.join(statuses)}]")
        return 0

    # Stable indices are based on the full sorted thread list.
    index_by_key = {thread_key(t): i for i, t in enumerate(all_threads)}
    for t in visible:
        idx = index_by_key.get(thread_key(t), -1)
        preview = latest_comment_preview(t)
        print(
            " ".join(
                p
                for p in [
                    f"#{idx}",
                    f"status={t.status}",
                    f"file={t.file_path}",
                    f"old={t.line_number_old if t.line_number_old is not None else 'x'}",
                    f"new={t.line_number_new if t.line_number_new is not None else 'x'}",
                    f"type={t.line_type}",
                    f"comments={len(t.comments)}",
                    (f'latest="{preview}"' if preview else ""),
                ]
                if p
            )
        )
    return 0


def map_back_and_write(
    store_path: Path,
    store: dict[str, Any],
    task_id: str,
    selected: Thread,
) -> None:
    tasks = store.get("tasks")
    if not isinstance(tasks, dict):
        raise RuntimeError("Invalid store: tasks")
    entry = tasks.get(task_id)
    if not isinstance(entry, dict):
        raise RuntimeError(f"Review task entry not found: {task_id}")
    threads_raw = entry.get("threads")
    if not isinstance(threads_raw, list):
        raise RuntimeError("Invalid store: threads")

    key = thread_key(selected)
    idx = -1
    for i, raw in enumerate(threads_raw):
        try:
            if thread_key(normalize_thread(raw)) == key:
                idx = i
                break
        except Exception:
            continue
    if idx == -1:
        raise RuntimeError(f"Failed to map normalized thread back into store: {key}")

    # Keep store camelCase shape (matches Rust/TS).
    threads_raw[idx] = {
        "filePath": selected.file_path,
        "lineNumberOld": selected.line_number_old,
        "lineNumberNew": selected.line_number_new,
        "lineType": selected.line_type,
        "status": selected.status,
        "comments": selected.comments,
    }
    entry["threads"] = threads_raw
    tasks[task_id] = entry
    store["tasks"] = tasks
    write_json(store_path, store)


def load_store(store_path: Path) -> dict[str, Any]:
    if store_path.exists():
        raw = read_json(store_path)
        return ensure_store(raw)
    store = ensure_store(None)
    write_json(store_path, store)
    return store


def select_thread(entry: dict[str, Any], thread_index: int) -> Thread:
    all_threads = list_threads(entry)
    if thread_index < 0 or thread_index >= len(all_threads):
        raise RuntimeError(
            f"Thread index out of range: {thread_index}. Use 'list' to see valid indices."
        )
    return all_threads[thread_index]


def main(argv: list[str]) -> int:
    if "--help" in argv or "-h" in argv:
        print(
            "Usage:\n"
            "  python3 illuc-review.py list [--task <uuid>] [--status active,pending]\n"
            "  python3 illuc-review.py add --file <path> [--line-new N|--line-old N] [--line-type add|del|context|meta|hunk] [--status active|pending|resolved|wont-fix|closed] --message \"<text>\" [--author <name>] [--task <uuid>]\n"
            "  python3 illuc-review.py comment --thread <n> [--task <uuid>] --message \"<text>\" [--author <name>]\n"
            "  python3 illuc-review.py resolve --thread <n> [--task <uuid>] --message \"<text>\" [--author <name>]\n"
            "  python3 illuc-review.py set-status --thread <n> --status <active|pending|resolved|wont-fix|closed> [--task <uuid>]\n"
            "\n"
            "Notes:\n"
            "  - Operates on <worktreeRoot>/.illuc/local-review.json (worktreeRoot is git toplevel).\n"
            "  - Task id is usually the worktree folder name (UUID). If not provided, we infer it.\n"
            "  - You can set a default author via env var ILLUC_REVIEW_AUTHOR.\n"
        )
        return 0

    parser = argparse.ArgumentParser(add_help=False)
    sub = parser.add_subparsers(dest="cmd")

    p_list = sub.add_parser("list")
    p_list.add_argument("--task", type=str, default=None)
    p_list.add_argument("--status", type=str, default=None)

    def add_thread_args(p: argparse.ArgumentParser) -> None:
        p.add_argument("--task", type=str, default=None)
        p.add_argument("--thread", type=int, required=True)

    p_comment = sub.add_parser("comment")
    add_thread_args(p_comment)
    p_comment.add_argument("--message", type=str, required=True)
    p_comment.add_argument("--author", type=str, default=None)

    p_resolve = sub.add_parser("resolve")
    add_thread_args(p_resolve)
    p_resolve.add_argument("--message", type=str, required=True)
    p_resolve.add_argument("--author", type=str, default=None)

    p_set = sub.add_parser("set-status")
    add_thread_args(p_set)
    p_set.add_argument(
        "--status",
        type=str,
        required=True,
        choices=["active", "pending", "resolved", "wont-fix", "closed"],
    )

    p_add = sub.add_parser("add")
    p_add.add_argument("--task", type=str, default=None)
    p_add.add_argument("--file", type=str, required=True)
    p_add.add_argument("--line-old", type=int, default=None)
    p_add.add_argument("--line-new", type=int, default=None)
    p_add.add_argument(
        "--line-type",
        type=str,
        default="context",
        choices=["add", "del", "context", "meta", "hunk"],
    )
    p_add.add_argument(
        "--status",
        type=str,
        default="active",
        choices=["active", "pending", "resolved", "wont-fix", "closed"],
    )
    p_add.add_argument("--message", type=str, required=True)
    p_add.add_argument("--author", type=str, default=None)

    p_help = sub.add_parser("--help")

    ns, extra = parser.parse_known_args(argv)
    if extra:
        # Keep behavior simple/strict.
        raise RuntimeError(f"Unexpected args: {' '.join(extra)}")
    if ns.cmd in (None, "--help"):
        print(
            "Usage:\n"
            "  python3 illuc-review.py list [--task <uuid>] [--status active,pending]\n"
            "  python3 illuc-review.py add --file <path> [--line-new N|--line-old N] [--line-type add|del|context|meta|hunk] [--status active|pending|resolved|wont-fix|closed] --message \"<text>\" [--author <name>] [--task <uuid>]\n"
            "  python3 illuc-review.py comment --thread <n> [--task <uuid>] --message \"<text>\" [--author <name>]\n"
            "  python3 illuc-review.py resolve --thread <n> [--task <uuid>] --message \"<text>\" [--author <name>]\n"
            "  python3 illuc-review.py set-status --thread <n> --status <active|pending|resolved|wont-fix|closed> [--task <uuid>]\n"
            "\n"
            "Notes:\n"
            "  - Operates on <worktreeRoot>/.illuc/local-review.json (worktreeRoot is git toplevel).\n"
            "  - Task id is usually the worktree folder name (UUID). If not provided, we infer it.\n"
            "  - You can set a default author via env var ILLUC_REVIEW_AUTHOR.\n"
        )
        return 0

    worktree_root = try_git_toplevel() or Path.cwd()
    store_path = worktree_root / ".illuc" / "local-review.json"
    store = load_store(store_path)
    task_id = infer_task_id(ns.task, worktree_root, store)

    if ns.cmd == "list":
        statuses = ["active", "pending"]
        if ns.status is not None:
            statuses = [s.strip() for s in ns.status.split(",") if s.strip()]
        return cmd_list(store_path, store, task_id, statuses)

    if not task_id:
        raise RuntimeError("Unable to infer task id. Pass --task <uuid>.")

    def resolve_author(explicit: Optional[str]) -> str:
        value = (explicit or "").strip()
        if value:
            return value
        env = (os.getenv("ILLUC_REVIEW_AUTHOR") or "").strip()
        if env:
            return env
        # Intentionally not "user" so the UI doesn't map it to the local user display name.
        return "agent"

    tasks = store.get("tasks") or {}
    entry = tasks.get(task_id)

    if ns.cmd == "add":
        file_path = str(ns.file).strip()
        if not file_path:
            raise RuntimeError("--file cannot be empty")
        line_old = ns.line_old
        line_new = ns.line_new
        if line_old is None and line_new is None:
            raise RuntimeError("Review thread must include --line-new or --line-old.")
        msg = str(ns.message).strip()
        if not msg:
            raise RuntimeError("--message cannot be empty")
        author = resolve_author(ns.author)

        if not isinstance(entry, dict):
            entry = {"taskId": task_id, "threads": []}
            tasks[task_id] = entry
            store["tasks"] = tasks

        threads_raw = entry.get("threads")
        if not isinstance(threads_raw, list):
            threads_raw = []
            entry["threads"] = threads_raw

        selected: Optional[Thread] = None
        for raw in threads_raw:
            try:
                t = normalize_thread(raw)
            except Exception:
                continue
            if (
                t.file_path == file_path
                and t.line_number_old == line_old
                and t.line_number_new == line_new
            ):
                selected = t
                break

        if selected is None:
            selected = Thread(
                file_path=file_path,
                line_number_old=line_old,
                line_number_new=line_new,
                line_type=str(ns.line_type),
                status=str(ns.status),
                comments=[],
            )
            threads_raw.append(
                {
                    "filePath": selected.file_path,
                    "lineNumberOld": selected.line_number_old,
                    "lineNumberNew": selected.line_number_new,
                    "lineType": selected.line_type,
                    "status": selected.status,
                    "comments": selected.comments,
                }
            )

        selected.comments.append(
            {
                "id": str(uuid.uuid4()),
                "body": msg,
                "author": author,
                "createdAt": iso_utc_now(),
            }
        )
        map_back_and_write(store_path, store, task_id, selected)
        print(f"add OK task={task_id} key={thread_key(selected)} store={store_path}")
        return 0

    if not isinstance(entry, dict):
        raise RuntimeError(f"Review task entry not found: {task_id}")

    thread = select_thread(entry, ns.thread)
    key = thread_key(thread)

    if ns.cmd == "comment":
        msg = str(ns.message).strip()
        if not msg:
            raise RuntimeError("--message cannot be empty")
        author = resolve_author(ns.author)
        thread.comments.append(
            {
                "id": str(uuid.uuid4()),
                "body": msg,
                "author": author,
                "createdAt": iso_utc_now(),
            }
        )
        map_back_and_write(store_path, store, task_id, thread)
        print(f"comment OK task={task_id} thread=#{ns.thread} key={key} store={store_path}")
        return 0

    if ns.cmd == "resolve":
        msg = str(ns.message).strip()
        if not msg:
            raise RuntimeError("--message cannot be empty")
        author = resolve_author(ns.author)
        thread.status = "resolved"
        thread.comments.append(
            {
                "id": str(uuid.uuid4()),
                "body": msg,
                "author": author,
                "createdAt": iso_utc_now(),
            }
        )
        map_back_and_write(store_path, store, task_id, thread)
        print(f"resolve OK task={task_id} thread=#{ns.thread} key={key} store={store_path}")
        return 0

    if ns.cmd == "set-status":
        status = str(ns.status).strip()
        if not status:
            raise RuntimeError("--status cannot be empty")
        thread.status = status
        map_back_and_write(store_path, store, task_id, thread)
        print(
            f"set-status OK task={task_id} thread=#{ns.thread} key={key} status={status} store={store_path}"
        )
        return 0

    raise RuntimeError("Unknown command")


if __name__ == "__main__":
    try:
        raise SystemExit(main(sys.argv[1:]))
    except BrokenPipeError:
        # Allow piping to head/rg/etc.
        raise
    except Exception as e:
        print(str(e), file=sys.stderr)
        raise SystemExit(1)
