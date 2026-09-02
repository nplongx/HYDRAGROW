#!/usr/bin/env python3
"""Validate that a HYDRAGROW PR body has the required delivery-contract sections
AND that each section has real content — not just a pasted header.

Reads the PR body from the PR_BODY environment variable (kept out of argv so it
never gets truncated/escaped by shell quoting).
"""
import os
import re
import sys

REQUIRED_SECTIONS = [
    "## Requirement",
    "## Objective",
    "## Acceptance Criteria",
    "## Verification",
    "## Documentation",
    "## Risks / Known Gaps",
    "## Final Acceptance",
]

# Minimum non-whitespace, non-bullet characters a section body must contain.
# Deliberately low: the goal is only to catch a section left completely blank
# (or with just a bullet/checkbox skeleton) — short-but-real answers like
# "ACCEPTED" or "None" are legitimate and must not be rejected.
MIN_CONTENT_CHARS = 3


def fail(messages: list[str]) -> None:
    print("PR contract is incomplete:")
    for m in messages:
        print(f" - {m}")
    raise SystemExit(1)


def main() -> None:
    body = os.environ.get("PR_BODY") or ""
    headers = [h.strip() for h in re.findall(r"^##\s.*$", body, flags=re.MULTILINE)]
    failures = []

    for section in REQUIRED_SECTIONS:
        if section not in headers:
            failures.append(f"missing section '{section}'")
            continue

        pattern = re.escape(section) + r"\s*\n(.*?)(?=\n##\s|\Z)"
        match = re.search(pattern, body, flags=re.DOTALL)
        content = match.group(1).strip() if match else ""
        # Strip markdown bullet/checkbox noise so "- [ ] " alone doesn't count.
        stripped = re.sub(r"[-*\[\]\s]", "", content)
        if len(stripped) < MIN_CONTENT_CHARS:
            failures.append(
                f"section '{section}' has no real content "
                f"({len(stripped)} chars after stripping bullets/checkboxes, "
                f"need >= {MIN_CONTENT_CHARS})"
            )

    if failures:
        fail(failures)

    print("PR contract sections present with real content.")


if __name__ == "__main__":
    main()
