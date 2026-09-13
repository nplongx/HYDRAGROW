# PR QA Checklist — UI Changes (`hydragrow-frontend`)

Source: `hydragrow-frontend/CHUAN-GIAO-DIEN-FRONTEND.md` §8
("Checklist trước khi merge PR có thay đổi UI"). Each item below is one §8
bullet converted to a yes/no question with a concrete verification command.
Run commands from the **repo root** (verify.yml `test_cmd` for the frontend
subsystem is `cd hydragrow-frontend && npx vitest run`).

Reusable guards live in `hydragrow-frontend/src/lib/design-lint/`
(added by Tasks 1–3). Allowed categorical exceptions are listed in
`hydragrow-frontend/src/lib/design-lint/colorAllowlist.json`
(automation node-type palette — deliberate, not drift).

---

## Checklist

### 1. No new hardcoded off-brand colors?

> §8: "Không có `bg-blue-*` / `text-blue-*` / `border-blue-*` mới
> (trừ khi đã đổi chuẩn ở mục 1.1)"

- [ ] `cd hydragrow-frontend && npx vitest run src/lib/design-lint/hardcodedColors.test.ts`
      — scans every `*.tsx` for `bg/text/border/ring/fill/stroke-{blue,indigo,
      purple,pink,violet,fuchsia,cyan,teal,orange,lime,rose}-{shades}` outside
      `colorAllowlist.json`. If it reports your file: either switch to the
      token (`--color-info-fg` for neutral/info instead of `blue`, `DeviceStatePill`
      for status instead of `rose`/`emerald` — see Task 1 Roles.tsx fix) or, if
      yours is a genuine categorical scheme like the automation node-type
      palette, add the path to the allowlist with a justification comment.
- [ ] NOTE: the guard still fails repo-wide for ~230 pre-existing,
      not-yet-classified matches (DevicePairing, ConfigBackup, RecipeBuilder,
      SensorBentoCard, …). Your bar: **your PR introduces zero NEW violations**
      — compare the failure list before/after your change.

### 2. New pages wrapped in shared layout classes?

> §8: "Trang mới bọc trong `.app-page`, dùng `.ui-card` cho các khối nội dung"

- [ ] `cd hydragrow-frontend && grep -rn "app-page" src/pages/<YourPage>.tsx`
      — the page root must be `<div className="app-page">`, content blocks
      `<section className="ui-card">`, header `.page-header*`
      (reference: Task 2 replaced orphan `farm-page-shell`/`farm-title`/
      `farm-subtitle` in Roles.tsx and DevicePairing.tsx with exactly these).
- [ ] `cd hydragrow-frontend && npx vitest run src/lib/design-lint/orphanClasses.test.ts`
      — fails if you referenced any `ui-*`/`farm-*` class never defined in
      `src/App.css`.

### 3. No new undefined `ui-*` / `farm-*` classes?

> §8: "Không tạo class `ui-btn-*` / `farm-*` mới mà không khai báo trong `App.css`"

- [ ] `cd hydragrow-frontend && npx vitest run src/lib/design-lint/orphanClasses.test.ts`
      — must PASS (it does since Task 2). If you add a class to `App.css`,
      also grep that no other file already hand-rolled the same look inline.

### 4. New route placed in exactly one of the 5 navigation groups?

> §8: "Route mới đã được xếp vào đúng 1 trong 5 nhóm điều hướng (mục 4)"
> (Tổng quan / Vận hành / Canh tác / Nhật ký / Cài đặt)

- [ ] Manual: name the group in the PR description and point to the tab/route
      entry. No automated guard — reviewer confirms no new "Thêm"-style
      catch-all bucket is reintroduced.

### 5. Checked at mobile AND `lg:` (desktop/Tauri) breakpoints?

> §8: "Đã kiểm tra hiển thị ở cả breakpoint mobile và `lg:` (desktop/Tauri)"

- [ ] `cd hydragrow-frontend && npm run build` — must exit clean (catches
      Tailwind/breakpoint typos at build time).
- [ ] Manual: screenshot or note for `<lg` (bottom pill tab bar intact) and
      `lg:`+ (sidebar layout, no `max-w-6xl` letterboxing of canvas pages).
      Reference: Task 5 changed the Settings tab bar to
      `grid grid-cols-2 sm:grid-cols-4` — verify both widths after any
      tab-bar edit.

### 6. Loading / empty / error states use the shared components?

> §8: "Trạng thái loading/rỗng/lỗi dùng component chung,
> không viết `div` tùy biến mới"

- [ ] Empty/unconfigured → `<StateView icon title description action?>`
      (`src/components/ui/StateView.tsx`); toggles → `<Switch>`
      (`src/components/ui/Switch.tsx`, assert via `getByRole('switch')`);
      status → `<DeviceStatePill>` (`src/components/ui/DeviceStatePill.tsx`).
      Full prop tables: `docs/design-system/PATTERN-LIBRARY.md`.
- [ ] `cd hydragrow-frontend && npx vitest run src/lib/design-lint/orphanClasses.test.ts`
      — catches bespoke `div` blocks that invent lookalike classes.

### 7. Text contrast still meets WCAG AA (4.5:1)?

> New guard from Task 3 (no §8 equivalent — `--color-faint` `#7c9385` failed
> at 2.75:1 on page-bg / 3.30:1 on white; darkened to `#556d5e`).

- [ ] `cd hydragrow-frontend && npx vitest run src/lib/design-lint/contrast.test.ts`
      — asserts the live `--color-faint` token in `App.css` clears 4.5:1 on
      both `--color-page-bg` and `--color-surface`. If you introduce a new
      text token, extend this test with the new pair before merge.

### 8. `Roles.tsx` `CAPABILITIES` matches `ROLE_DEFAULT_SCOPES`?

> NOT in §8 — added after the Discrepancy 3 fix
> (`fix(roles): align capability matrix with ROLE_DEFAULT_SCOPES`).

- [ ] Manual diff: `CAPABILITIES` (`src/pages/Roles.tsx:42`) vs
      `ROLE_DEFAULT_SCOPES` (`src/pages/Roles.tsx:21`). No automated guard
      exists yet — every role key in one table must have a consistent entry
      in the other. If you touch either table, paste the diff in the PR.

### 9. **Hardcoded-color tracked debt** — nếu PR của bạn chạm vào
   `SensorBentoCard.tsx`, `ConfigBackup.tsx`, hoặc `RecipeBuilder.tsx`,
   đây là lúc dọn luôn drift màu của đúng file đó (xoá entry tương ứng
   khỏi `colorAllowlist.json` trong cùng PR) — đừng để lại cho người
   sau. Ba file này được ghi nợ có chủ đích trong
   `2026-09-13-layer2-design-ux-agentteams-rollout.md`, không phải
   miễn trừ vĩnh viễn.
