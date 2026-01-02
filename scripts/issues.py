#!/usr/bin/env python3
"""
Single entrypoint for GitHub issue/project helpers (replaces multiple scripts).

Requires: GitHub CLI `gh` authenticated. Reads repo settings from env or git.

Usage:
  issues.py start <number>            # assign self, add in-progress, create branch
  issues.py mirror                    # write docs/issues/index.json mirror
  issues.py mirror --check            # exit non-zero if mirror stale vs GitHub

Env (optional):
  AIDEON_GH_REPO=owner/repo
"""
from __future__ import annotations
import argparse
import json
import os
import re
import subprocess
import sys
import textwrap
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
DOCS_ISSUES = ROOT / "docs" / "issues"
MIRROR_FILE = DOCS_ISSUES / "index.json"


def run(cmd: list[str], check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, check=check, text=True, capture_output=True)


def repo_slug() -> str:
    env_repo = os.environ.get("AIDEON_GH_REPO")
    if env_repo:
        return env_repo
    # Derive from git remote origin
    try:
        out = run(["git", "remote", "get-url", "origin"]).stdout.strip()
        m = re.search(r"[:/]([^/]+/[^/.]+)(?:\.git)?$", out)
        if m:
            return m.group(1)
    except Exception:
        pass
    raise SystemExit("AIDEON_GH_REPO not set and cannot derive from git remote")


def gh_json(args: list[str]) -> Any:
    cmd = ["gh", *args]
    res = run(cmd, check=True)
    return json.loads(res.stdout)


def gh_graphql(query: str, variables: dict[str, Any]) -> dict[str, Any]:
    args = ["gh", "api", "graphql", "-f", f"query={textwrap.dedent(query).strip()}"]
    for key, value in variables.items():
        if value is None:
            continue
        if isinstance(value, bool):
            rendered = "true" if value else "false"
        else:
            rendered = str(value)
        args.extend(["-F", f"{key}={rendered}"])
    res = run(args, check=True)
    payload = json.loads(res.stdout or "{}")
    if "errors" in payload:
        raise SystemExit(f"GraphQL query failed: {payload['errors']}")
    data = payload.get("data")
    if not isinstance(data, dict):
        raise SystemExit("GraphQL response missing data field")
    return data


def cmd_start(args: argparse.Namespace) -> int:
    issue = str(args.number)
    repo = repo_slug()
    # Assign self and add in-progress label
    run(["gh", "issue", "edit", issue, "--repo", repo, "--add-assignee", "@me"], check=True)
    run(
        [
            "gh",
            "issue",
            "edit",
            issue,
            "--repo",
            repo,
            "--add-label",
            "status/in-progress",
        ],
        check=False,
    )
    # Get title for branch slug
    meta = gh_json(["issue", "view", issue, "--repo", repo, "--json", "title,number"])
    slug = re.sub(r"[^a-z0-9]+", "-", meta["title"].lower()).strip("-")
    branch = f"issue-{meta['number']}-{slug}"[:80]
    # Create/switch branch
    run(["git", "checkout", "-B", branch], check=True)
    print(f"[issues] started issue #{issue} on branch {branch}")
    return 0


def fetch_issues(repo: str) -> list[dict[str, Any]]:
    # Pull basic fields for mirror/check
    data = gh_json(
        [
            "issue",
            "list",
            "--repo",
            repo,
            "--state",
            "all",
            "--limit",
            "200",
            "--json",
            "number,title,state,labels,updatedAt,assignees,milestone",
        ]
    )
    # Normalize labels to names only
    for it in data:
        it["labels"] = sorted(
            [label.get("name") for label in it.get("labels", []) if isinstance(label, dict)]
        )
    return data


def cmd_mirror(args: argparse.Namespace) -> int:
    repo = repo_slug()
    try:
        live = fetch_issues(repo)
    except subprocess.CalledProcessError as exc:
        if args.check:
            # Non-blocking in CI/pre-push when gh is unavailable
            print(f"[issues] warn: gh failed in --check: {exc}", file=sys.stderr)
            return 0
        raise
    DOCS_ISSUES.mkdir(parents=True, exist_ok=True)
    if args.check:
        if not MIRROR_FILE.exists():
            print("[issues] mirror missing", file=sys.stderr)
            return 1
        try:
            cached = json.loads(MIRROR_FILE.read_text())
        except Exception:
            print("[issues] mirror unreadable", file=sys.stderr)
            return 1
        cached_map = {int(x["number"]): x for x in cached}
        changed = False
        for it in live:
            n = int(it["number"])
            c = cached_map.get(n)
            if (
                c is None
                or c.get("updatedAt") != it.get("updatedAt")
                or c.get("state") != it.get("state")
            ):
                changed = True
                break
        if changed:
            print("[issues] mirror stale", file=sys.stderr)
            return 1
        print("[issues] mirror up to date")
        return 0
    else:
        MIRROR_FILE.write_text(json.dumps(live, indent=2))
        print(f"[issues] wrote {MIRROR_FILE}")
        return 0


def project_env() -> tuple[str, int, str, dict[str, str]]:
    owner = os.environ.get("AIDEON_GH_PROJECT_OWNER")
    if not owner:
        raise SystemExit("AIDEON_GH_PROJECT_OWNER must be set (org or user login)")
    try:
        number = int(os.environ.get("AIDEON_GH_PROJECT_NUMBER", ""))
    except ValueError as exc:
        raise SystemExit("AIDEON_GH_PROJECT_NUMBER must be an integer") from exc
    if not number:
        raise SystemExit("AIDEON_GH_PROJECT_NUMBER must be provided")
    status_field = os.environ.get("AIDEON_GH_STATUS_FIELD")
    if not status_field:
        raise SystemExit("AIDEON_GH_STATUS_FIELD must be set (e.g. 'Status')")
    raw_map = os.environ.get("AIDEON_GH_STATUS_MAP")
    if not raw_map:
        raise SystemExit("AIDEON_GH_STATUS_MAP must map labels to project statuses")
    try:
        mapping: dict[str, str] = json.loads(raw_map)
    except json.JSONDecodeError as exc:
        raise SystemExit("AIDEON_GH_STATUS_MAP must be valid JSON") from exc
    if not isinstance(mapping, dict) or not mapping:
        raise SystemExit("AIDEON_GH_STATUS_MAP must be a non-empty JSON object")
    normalized: dict[str, str] = {}
    for label, status in mapping.items():
        if not isinstance(label, str) or not isinstance(status, str):
            raise SystemExit("AIDEON_GH_STATUS_MAP must map strings to strings")
        normalized[label] = status
    return owner, number, status_field, normalized


def fetch_project(owner: str, number: int, status_field: str) -> tuple[str, str, dict[str, str]]:
    query = """
    query($owner: String!, $number: Int!) {
      organization(login: $owner) {
        projectV2(number: $number) {
          id
          fields(first: 50) {
            nodes {
              ... on ProjectV2SingleSelectField {
                id
                name
                options {
                  id
                  name
                }
              }
            }
          }
        }
      }
      user(login: $owner) {
        projectV2(number: $number) {
          id
          fields(first: 50) {
            nodes {
              ... on ProjectV2SingleSelectField {
                id
                name
                options {
                  id
                  name
                }
              }
            }
          }
        }
      }
    }
    """
    data = gh_graphql(query, {"owner": owner, "number": number})
    project = None
    if data.get("organization", {}).get("projectV2"):
        project = data["organization"]["projectV2"]
    elif data.get("user", {}).get("projectV2"):
        project = data["user"]["projectV2"]
    if project is None:
        raise SystemExit(
            f"Project {owner}/{number} not found. Ensure credentials grant project access."
        )
    project_id = project.get("id")
    if not project_id:
        raise SystemExit("Project response missing id")
    status_field_id = None
    options: dict[str, str] = {}
    for field in project.get("fields", {}).get("nodes", []):
        if field and field.get("name") == status_field:
            status_field_id = field.get("id")
            options = {opt.get("name"): opt.get("id") for opt in field.get("options", []) if opt}
            break
    if not status_field_id:
        raise SystemExit(f"Project missing status field '{status_field}'")
    if not options:
        raise SystemExit(f"Project status field '{status_field}' has no options")
    return project_id, status_field_id, options


def fetch_project_items(project_id: str, status_field_id: str) -> dict[str, dict[str, str | None]]:
    query = """
    query($project: ID!, $after: String) {
      node(id: $project) {
        ... on ProjectV2 {
          items(first: 100, after: $after) {
            nodes {
              id
              content {
                __typename
                ... on Issue {
                  id
                  number
                }
              }
              fieldValues(first: 20) {
                nodes {
                  ... on ProjectV2ItemFieldSingleSelectValue {
                    field {
                      id
                    }
                    optionId
                  }
                }
              }
            }
            pageInfo {
              hasNextPage
              endCursor
            }
          }
        }
      }
    }
    """
    cursor: str | None = None
    items: dict[str, dict[str, str | None]] = {}
    while True:
        variables = {"project": project_id, "after": cursor}
        data = gh_graphql(query, variables)
        node = data.get("node")
        if node is None:
            break
        connection = node.get("items", {})
        for entry in connection.get("nodes", []):
            if not entry:
                continue
            content = entry.get("content") or {}
            if content.get("__typename") != "Issue":
                continue
            content_id = content.get("id")
            if not content_id:
                continue
            status_value = None
            for fv in (entry.get("fieldValues") or {}).get("nodes", []):
                if fv and fv.get("field", {}).get("id") == status_field_id:
                    status_value = fv.get("optionId")
                    break
            items[content_id] = {
                "item_id": entry.get("id"),
                "status_option": status_value,
            }
        page = connection.get("pageInfo") or {}
        if page.get("hasNextPage"):
            cursor = page.get("endCursor")
        else:
            break
    return items


def fetch_repo_issues(owner: str, name: str) -> list[dict[str, Any]]:
    query = """
    query($owner: String!, $name: String!, $after: String) {
      repository(owner: $owner, name: $name) {
        issues(first: 100, after: $after, states: [OPEN, CLOSED]) {
          nodes {
            id
            number
            labels(first: 50) {
              nodes {
                name
              }
            }
          }
          pageInfo {
            hasNextPage
            endCursor
          }
        }
      }
    }
    """
    issues: list[dict[str, Any]] = []
    cursor: str | None = None
    while True:
        variables = {"owner": owner, "name": name, "after": cursor}
        data = gh_graphql(query, variables)
        repo = data.get("repository")
        if repo is None:
            raise SystemExit(f"Repository {owner}/{name} not accessible via GraphQL")
        conn = repo.get("issues") or {}
        for node in conn.get("nodes", []):
            if not node:
                continue
            labels = [lbl.get("name") for lbl in (node.get("labels") or {}).get("nodes", []) if lbl]
            node["labels"] = labels
            issues.append(node)
        page = conn.get("pageInfo") or {}
        if page.get("hasNextPage"):
            cursor = page.get("endCursor")
        else:
            break
    return issues


def add_project_item(project_id: str, content_id: str) -> str:
    mutation = """
    mutation($projectId: ID!, $contentId: ID!) {
      addProjectV2ItemById(input: {projectId: $projectId, contentId: $contentId}) {
        item {
          id
        }
      }
    }
    """
    data = gh_graphql(mutation, {"projectId": project_id, "contentId": content_id})
    item = data.get("addProjectV2ItemById", {}).get("item")
    if not item or not item.get("id"):
        raise SystemExit("Failed to add issue to project board")
    return item["id"]


def update_project_status(
    project_id: str,
    item_id: str,
    field_id: str,
    option_id: str,
) -> None:
    mutation = """
    mutation($projectId: ID!, $itemId: ID!, $fieldId: ID!, $optionId: String!) {
      updateProjectV2ItemFieldValue(
        input: {
          projectId: $projectId,
          itemId: $itemId,
          fieldId: $fieldId,
          value: {singleSelectOptionId: $optionId}
        }
      ) {
        projectV2Item {
          id
        }
      }
    }
    """
    gh_graphql(
        mutation,
        {
            "projectId": project_id,
            "itemId": item_id,
            "fieldId": field_id,
            "optionId": option_id,
        },
    )


def cmd_project(args: argparse.Namespace) -> int:
    repo = repo_slug()
    try:
        owner_repo = repo.split("/", 1)
        repo_owner, repo_name = owner_repo[0], owner_repo[1]
    except IndexError as exc:
        raise SystemExit(f"Invalid repo slug: {repo}") from exc
    project_owner, project_number, status_field, label_status_map = project_env()
    project_id, status_field_id, status_options = fetch_project(
        project_owner, project_number, status_field
    )
    ordered_mapping = list(label_status_map.items())
    label_to_option: list[tuple[str, str]] = []
    for label, status_name in ordered_mapping:
        option_id = status_options.get(status_name)
        if not option_id:
            raise SystemExit(
                f"Status '{status_name}' from AIDEON_GH_STATUS_MAP not found in project options"
            )
        label_to_option.append((label, option_id))

    existing_items = fetch_project_items(project_id, status_field_id)
    issues = fetch_repo_issues(repo_owner, repo_name)
    added = 0
    updated = 0
    for issue in issues:
        content_id = issue.get("id")
        if not content_id:
            continue
        labels = set(issue.get("labels", []))
        target_option = None
        for label, option in label_to_option:
            if label in labels:
                target_option = option
                break
        item_entry = existing_items.get(content_id)
        item_id = item_entry.get("item_id") if item_entry else None
        current_option = item_entry.get("status_option") if item_entry else None
        if item_id is None:
            if args.dry_run:
                print(f"[issues] dry-run: would add issue #{issue.get('number')} to project")
            else:
                item_id = add_project_item(project_id, content_id)
            if item_id:
                existing_items[content_id] = {
                    "item_id": item_id,
                    "status_option": current_option,
                }
                added += 1
        if target_option and item_id:
            if current_option != target_option:
                if args.dry_run:
                    print(
                        "[issues] dry-run: would set status of issue #"
                        f"{issue.get('number')} to option {target_option}"
                    )
                else:
                    update_project_status(project_id, item_id, status_field_id, target_option)
                existing_items[content_id]["status_option"] = target_option
                updated += 1
    print(
        f"[issues] project sync complete: {added} added, {updated} status updates"
        + (" (dry-run)" if args.dry_run else "")
    )
    return 0


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(prog="issues")
    sub = ap.add_subparsers(dest="cmd", required=True)

    ap_start = sub.add_parser("start", help="assign, label, create/switch branch")
    ap_start.add_argument("number", type=int)
    ap_start.set_defaults(func=cmd_start)

    ap_mirror = sub.add_parser("mirror", help="mirror issues to docs/issues/index.json")
    ap_mirror.add_argument("--check", action="store_true", help="check if mirror is stale")
    ap_mirror.set_defaults(func=cmd_mirror)

    ap_project = sub.add_parser(
        "project",
        help="sync project status with issue labels (requires GitHub Projects access)",
    )
    ap_project.add_argument("--dry-run", action="store_true", help="log actions without mutating")
    ap_project.set_defaults(func=cmd_project)

    # Split: create child issues from items or a file and update parent checklist
    def cmd_split(ns: argparse.Namespace) -> int:
        repo = repo_slug()
        parent = str(ns.parent)
        items: list[str] = []
        if ns.items:
            items.extend(ns.items)
        if ns.file:
            fp = Path(ns.file)
            items.extend([ln.strip() for ln in fp.read_text().splitlines() if ln.strip()])
        if not items:
            print("[issues] no items provided", file=sys.stderr)
            return 1
        created: list[tuple[int, str]] = []
        for title in items:
            out = run(
                [
                    "gh",
                    "issue",
                    "create",
                    "--repo",
                    repo,
                    "--title",
                    title,
                    "--body",
                    f"Parent: #{parent}",
                    "--label",
                    "status/todo",
                    "--label",
                    "type/task",
                ],
                check=True,
            )
            m = re.search(r"/issues/(\d+)$", (out.stdout or out.stderr))
            if not m:
                print(f"[issues] created: {out.stdout.strip()}")
                continue
            num = int(m.group(1))
            created.append((num, title))
        # Update parent body with a checklist
        meta = gh_json(["issue", "view", parent, "--repo", repo, "--json", "body"])
        body = meta.get("body") or ""
        checklist = "\n".join([f"- [ ] #{n} {t}" for n, t in created])
        new_body = (
            body + ("\n\n### Subtasks\n" if "### Subtasks" not in body else "\n") + checklist + "\n"
        )
        run(["gh", "issue", "edit", parent, "--repo", repo, "--body", new_body], check=True)
        print(f"[issues] created {len(created)} subtasks under #{parent}")
        return 0

    ap_split = sub.add_parser("split", help="create child issues and update parent checklist")
    ap_split.add_argument("parent", type=int)
    ap_split.add_argument("--items", nargs="*", help="child issue titles")
    ap_split.add_argument("--file", help="file with one title per line")
    ap_split.set_defaults(func=cmd_split)

    # DoD: ensure a Definition of Done section exists
    DOD = (
        "\n\n### Definition of Done\n"
        "- CI: lint, typecheck, unit tests updated\n"
        "- Docs: user & dev docs updated (README/CHANGELOG)\n"
        "- Security: renderer IPC boundaries respected; no new ports\n"
        "- Performance: SLO notes or benches if applicable\n"
        "- UX: matches GitHub-inspired style (light/dark)\n"
        "- Packaging: macOS build verified (DMG/ZIP)\n"
        "- Tracking: PRs linked; Project Status updated; local mirror refreshed\n"
    )

    def cmd_dod(ns: argparse.Namespace) -> int:
        repo = repo_slug()
        issues = gh_json(
            [
                "issue",
                "list",
                "--repo",
                repo,
                "--label",
                "status/in-progress",
                "--state",
                "open",
                "--json",
                "number,body",
            ]
        )
        changed = 0
        for it in issues:
            n = str(it["number"])
            body = it.get("body") or ""
            if "### Definition of Done" in body:
                continue
            run(["gh", "issue", "edit", n, "--repo", repo, "--body", body + DOD], check=True)
            changed += 1
        print(f"[issues] ensured DoD on {changed} issues")
        return 0

    ap_dod = sub.add_parser("dod", help="ensure Definition of Done on in-progress issues")
    ap_dod.set_defaults(func=cmd_dod)

    # Linkify: ensure issues referenced in PR bodies have a comment with the PR link
    def cmd_linkify(ns: argparse.Namespace) -> int:
        repo = repo_slug()
        prs = gh_json(
            [
                "pr",
                "list",
                "--repo",
                repo,
                "--state",
                "open",
                "--json",
                "number,title,body,updatedAt",
            ]
        )
        count = 0
        for pr in prs:
            body = pr.get("body") or ""
            num = pr.get("number")
            matches = set(map(int, re.findall(r"#(\d+)", body)))
            for issue_number in matches:
                url = f"https://github.com/{repo}/pull/{num}"
                comment = f"Linked PR: #{num} — {url}"
                run(
                    [
                        "gh",
                        "issue",
                        "comment",
                        str(issue_number),
                        "--repo",
                        repo,
                        "--body",
                        comment,
                    ],
                    check=False,
                )
                count += 1
        print(f"[issues] linkified {count} references from PRs")
        return 0

    ap_linkify = sub.add_parser("linkify", help="comment on issues with links to recent PRs")
    ap_linkify.set_defaults(func=cmd_linkify)

    # Backfill: comment on issues referenced in commits since a date; optionally close on Fixes/Closes/Resolves
    def cmd_backfill(ns: argparse.Namespace) -> int:
        repo = repo_slug()
        since = ns.since
        log = run(["git", "log", f"--since={since}", "--pretty=%H %s"]).stdout.splitlines()
        count = 0
        for line in log:
            mnums = set(map(int, re.findall(r"#(\d+)", line)))
            sha = line.split(" ", 1)[0]
            for issue_number in mnums:
                url = f"https://github.com/{repo}/commit/{sha}"
                comment = f"Referenced by commit {sha}: {url}"
                run(
                    [
                        "gh",
                        "issue",
                        "comment",
                        str(issue_number),
                        "--repo",
                        repo,
                        "--body",
                        comment,
                    ],
                    check=False,
                )
                if ns.close and re.search(r"\b(Fixes|Closes|Resolves)\b", line, re.IGNORECASE):
                    run(["gh", "issue", "close", str(issue_number), "--repo", repo], check=False)
                count += 1
        print(f"[issues] backfilled {count} references since {since}")
        return 0

    ap_backfill = sub.add_parser(
        "backfill", help="comment on issues referenced in commits; optionally close"
    )
    ap_backfill.add_argument("--since", required=True, help="ISO date (YYYY-MM-DD)")
    ap_backfill.add_argument(
        "--close", action="store_true", help="close issues on Fixes/Closes/Resolves"
    )
    ap_backfill.set_defaults(func=cmd_backfill)

    ns = ap.parse_args(argv)
    return ns.func(ns)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
