## Summary
-

## Reviewer checklist (critical areas)
- [ ] Pre-commit checks pass locally (`pre-commit run --all-files`).
- [ ] No new alias naming violations (`./scripts/check_alias_naming.sh`).
- [ ] Backend API/service/db changes include tests or clear validation notes.
- [ ] Firmware/controller changes include hardware safety impact notes.
- [ ] Migration changes are backward-compatible and rollback-aware.
- [ ] Frontend control/context changes include state transition review.
- [ ] Security/privacy impact reviewed (auth, secrets, device control path).
- [ ] Technical inconsistency impact is stated (increase/decrease/no-change).

## Quality metrics impact
- Lint violations (before -> after):
- Alias naming violations (before -> after):
- Technical inconsistency notes:

## Deployment risk
- [ ] Low
- [ ] Medium
- [ ] High

## Post-merge follow-ups
-
