"""Fail the build if a PR touches a path listed in .agent/verify.yml's restricted_files.

Reads the restricted_files list without a YAML dependency (it is a flat block of
quoted glob strings), so this check needs no extra pip install step in CI.
"""
import fnmatch
import re
import sys


def load_restricted_patterns(manifest_path):
    patterns = []
    in_block = False
    with open(manifest_path, "r") as f:
        for line in f:
            stripped = line.strip()
            if stripped.startswith("restricted_files:"):
                in_block = True
                continue
            if in_block:
                if not stripped or stripped.startswith("#"):
                    continue
                match = re.match(r'^-\s*"([^"]+)"', stripped)
                if match:
                    patterns.append(match.group(1))
                    continue
                break  # first non-list, non-comment line ends the block

    return patterns


def matches(path, pattern):
    if fnmatch.fnmatch(path, pattern):
        return True
    # fnmatch's "*" already crosses "/", but a literal "/" inside the pattern
    # (from a "**/" prefix) still has to appear in the path. Also try the
    # pattern with that prefix stripped so "**/*.key" matches a root-level
    # "secret.key", not only "sub/dir/secret.key".
    if pattern.startswith("**/"):
        return fnmatch.fnmatch(path, pattern[3:])
    return False


def main():
    if len(sys.argv) < 3:
        print("Usage: python3 check_protected_paths.py <verify.yml> <changed-files-file>")
        sys.exit(1)

    manifest_path, changed_files_path = sys.argv[1], sys.argv[2]
    patterns = load_restricted_patterns(manifest_path)
    if not patterns:
        print(f"No restricted_files patterns found in {manifest_path} — nothing to check.")
        return

    with open(changed_files_path, "r") as f:
        changed = [line.strip() for line in f if line.strip()]

    violations = [(path, p) for path in changed for p in patterns if matches(path, p)]

    if violations:
        print("Protected paths touched in this PR:")
        for path, pattern in violations:
            print(f"  - {path}  (matches restricted pattern: {pattern})")
        print(
            "\nThese paths require an explicit, human-reviewed change "
            f"(see restricted_files in {manifest_path}). "
            "Split them into a separate, manually reviewed PR."
        )
        sys.exit(1)

    print(f"No protected paths touched. Checked {len(changed)} file(s) against {len(patterns)} pattern(s).")


if __name__ == "__main__":
    main()
