# Layer 2 Design/UX Rollout (state-machine, error-handling-ux, data-visualization, form-design, search-ux, zeigarnik/peak-end) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **This plan is dual-runnable**: Section 1 also expresses the same task graph as a `dsh-agent-teams` profile, so a team captained by DeepSeek Harness can execute Tracks A–E in parallel instead. Whichever harness runs it, the source of truth for *what to build* is the Task list in Section 3 — the YAML in Section 1 only assigns owners and gates, it does not redefine the work.

**Goal:** Ship the six Lớp 2 items from the original roadmap — `state-machine` (pill 3 trạng thái dosing), `error-handling-ux` (banner/lock note), `data-visualization` (D16 Dosing History, D18 Analytics), `form-design` (D9 Pairing), `search-ux` (Journal), `zeigarnik-effect`/`peak-end-rule` (Seasons) — as real, tested code changes against the actual `hydragrow-frontend` codebase, at a larger scope than the original one-line bullets: each item becomes a full slice (new state/component + refactor of every real call site + regression tests + design-system doc updates), and every genuine bug discovered while reading the code for this plan gets fixed in the same PR as the track that touches that file (per `docs/design-system/GOVERNANCE.md` rule (c): "standard bugfix if a guard test exists").

**Architecture:** Five independent tracks (A–E) touch disjoint page domains (dosing controls, dosing history/analytics, device pairing, journal/system log, crop seasons) and can run in parallel once a one-time baseline task (Task 0) lands. Track A produces a shared `pumpVisualTheme.ts` palette module that Track B imports (so B depends on A). Track D produces its own `eventCategoryTheme.ts`, independent of A. A final Track F integrates docs and runs the full verification suite once A–E are all merged.

**Tech Stack:** React 19 + TypeScript (strict), Tailwind v4 tokens (`src/App.css`), Vitest + Testing Library (co-located tests), TanStack Query v5, Zustand v5 — no new runtime dependency is introduced anywhere in this plan (verified against `hydragrow-frontend/package.json`, which currently has zero charting library; all data-viz work in this plan is hand-rolled SVG/DOM, consistent with the existing `DosingTotalCard.tsx` pattern, not a new dependency).

**Baseline verified on `main` @ `b9eaac93` (2026-09-13), before this plan touches anything:**

```bash
cd hydragrow-frontend && npm ci && npx vitest run src/lib/design-lint/
```
```
Test Files  1 failed | 2 passed (3)
     Tests  1 failed | 5 passed (6)
```
`hardcodedColors.test.ts` is **already red on `main`**, independent of this plan, with 21 files in violation (full list in Task 0). This is not something this plan broke — it is the actual starting point, and Task 0 makes it an explicit, auditable starting point instead of a silent one.

---

## 1. Đội hình & DAG (tuỳ chọn: chạy qua `dsh-agent-teams`)

Nếu muốn chạy song song 5 track qua `NanmiCoder/dsh-agent-teams` (DeepSeek Harness) thay vì tuần tự trong một phiên Claude, dán khối dưới vào `cordis.patch.yml` của profile đang dùng, rồi gọi
`/agent-teams --profile hydragrow-layer2 ship the Layer 2 design/UX rollout`.
`taskPlanning: seed` được dùng có chủ đích — DAG đã cố định theo Section 3 của chính plan này, không để Captain tự suy diễn lại phạm vi.

```yaml
profiles:
  hydragrow-layer2:
    description: >
      HydraGrow frontend — Layer 2 design/UX rollout (state-machine,
      error-handling-ux, data-visualization, form-design, search-ux,
      zeigarnik/peak-end). Roster mirrors quality-gates.md's
      requirements→implementation→verification→review→integration
      contract; requirements are pre-resolved by this plan document, so
      each implementer starts directly at implementation.
    taskPlanning: seed
    protocol: >
      Follow docs/superpowers/plans/2026-09-13-layer2-design-ux-agentteams-rollout.md
      task-by-task. Do not invent scope beyond the six Layer 2 tracks.
      Do not touch files outside your assigned track's file perimeter
      (AGENTS.md §3, "positive scope"). Run the verification command
      listed in your track before marking a task completed.
    members:
      - name: baseline-lead
        model: deepseek-v4
        role: >
          Task 0 only: classify every hardcodedColors.test.ts violation on
          main, add tracked-debt allowlist entries for out-of-scope files,
          document the decision in GOVERNANCE.md.
      - name: dosing-control-lead
        model: deepseek-v4
        role: >
          Track A (Tasks 1-6): dosing pump state machine, 3-state pill,
          shared pump-identity palette, lock-note banner refactor.
          File perimeter: src/components/control/, src/components/ui/ControlCard.tsx,
          src/components/ui/StatusPill.tsx, src/pages/ControlPanel.tsx,
          src/lib/dosing/.
      - name: dataviz-lead
        model: deepseek-v4
        role: >
          Track B (Tasks 7-12): per-pump hourly chart, health sparklines.
          File perimeter: src/pages/DosingHistory.tsx, src/pages/Analytics.tsx,
          src/components/dosing/, src/lib/dosing/, src/components/ui/Sparkline.tsx.
      - name: pairing-formux-lead
        model: deepseek-v4
        role: >
          Track C (Tasks 13-16): device pairing form validation and error UX.
          File perimeter: src/pages/DevicePairing.tsx, src/components/pairing/.
      - name: journal-searchux-lead
        model: deepseek-v4
        role: >
          Track D (Tasks 17-19): journal/system-log search UX.
          File perimeter: src/pages/SystemLog.tsx, src/components/logs/.
      - name: seasons-motivation-lead
        model: deepseek-v4
        role: >
          Track E (Tasks 20-23): season progress checklist (Zeigarnik) and
          season-end summary (Peak-End).
          File perimeter: src/pages/CropSeasons.tsx, src/components/seasons/,
          src/lib/seasons/, src/components/recipes/ActiveRecipeStatus.tsx.
      - name: design-system-reviewer
        model: deepseek-v4
        role: >
          scope-reviewer + correctness-reviewer per quality-gates.md: for
          each track's diff, check changed_paths stay inside that track's
          declared perimeter, confirm docs/design-system/PATTERN-LIBRARY.md
          got a new entry for every new shared component, confirm no
          existing test's asserted copy string changed without a matching
          update to that test.
      - name: integrator
        model: deepseek-v4
        role: >
          Track F (Tasks 24-26): merge order, full `npx tsc --noEmit &&
          npx eslint . && npx vitest run` + `cargo check` in src-tauri,
          final docs sync (PATTERN-LIBRARY, GOVERNANCE, PR-QA-CHECKLIST,
          DESIGN.md §15, CHUAN-GIAO-DIEN-FRONTEND.md).
    tasks:
      - id: baseline
        subject: "Task 0 — classify hardcodedColors violations, tracked-debt allowlist"
        assignee: baseline-lead
      - id: track-a
        subject: "Tasks 1-6 — dosing state machine + pill + palette + lock banner"
        assignee: dosing-control-lead
        dependencies: [baseline]
      - id: track-b
        subject: "Tasks 7-12 — dosing/analytics data visualization"
        assignee: dataviz-lead
        dependencies: [track-a]
      - id: track-c
        subject: "Tasks 13-16 — device pairing form-design"
        assignee: pairing-formux-lead
        dependencies: [baseline]
      - id: track-d
        subject: "Tasks 17-19 — journal/system-log search-ux"
        assignee: journal-searchux-lead
        dependencies: [baseline]
      - id: track-e
        subject: "Tasks 20-23 — seasons Zeigarnik checklist + Peak-End summary"
        assignee: seasons-motivation-lead
        dependencies: [baseline]
      - id: review
        subject: "Scope + correctness review of tracks A-E"
        assignee: design-system-reviewer
        dependencies: [track-a, track-b, track-c, track-d, track-e]
      - id: integration
        subject: "Task 24-26 — docs sync + full-suite verification"
        assignee: integrator
        dependencies: [review]
```

**Chạy trong Claude thay vì DSH:** bỏ qua YAML trên, dùng `superpowers:subagent-driven-development` với cùng 5 track làm 5 subagent song song (mỗi subagent nhận đúng "File perimeter" ghi trên), review giữa các task theo đúng tinh thần `design-system-reviewer`, rồi một phiên cuối chạy Task 24–26. Không có gì trong Section 3 phụ thuộc vào việc dùng `dsh-agent-teams` — YAML chỉ là một cách lập lịch thay thế cho cùng một danh sách task.

---

## 2. Quy ước dùng chung cho mọi task dưới đây

- Lệnh xác minh chuẩn cho mọi task (trừ khi task ghi khác):
  ```bash
  cd hydragrow-frontend
  npx vitest run <đường dẫn test vừa sửa/thêm>
  npx tsc --noEmit
  ```
- Sau khi cả 5 track xong, Task 26 chạy bộ đầy đủ (`npx eslint .`, toàn bộ `npx vitest run`, `cargo check` trong `src-tauri`).
- Mọi màu mới đều lấy từ token đã có trong `src/App.css` (liệt kê ở đầu file); **không** thêm token mới nếu một token cũ đã đúng ngữ nghĩa (đúng tinh thần GOVERNANCE.md mục (b): token mới cần `contrast.test.ts` đi kèm và review Figma — plan này cố tình tránh mở rộng phạm vi đó).
- Copy tiếng Việt giữ nguyên văn phong đã có trong file (không Anh hoá, không đổi giọng văn).
- Mỗi task kết thúc bằng `git add` + `git commit` với message theo convention đã thấy trong `git log` của repo (`type(scope): mô tả ngắn`).

---

## 3. Task 0 — Baseline: phân loại vi phạm `hardcodedColors`, khoanh vùng nợ kỹ thuật

**Files:**
- Modify: `hydragrow-frontend/src/lib/design-lint/colorAllowlist.json`
- Modify: `docs/design-system/GOVERNANCE.md`
- Modify: `docs/design-system/PR-QA-CHECKLIST.md`

**Bối cảnh:** `npx vitest run src/lib/design-lint/` trên `main` hiện fail với 21 file vi phạm (chạy thật, xem đầu file). Theo GOVERNANCE.md mục (c), khi một guard đã tồn tại và đang fail, việc đúng là **phân loại từng file**: "categorical palette cần allowlist" hay "drift cần fix token" — không được im lặng nới lỏng test. Track A–E dưới đây sẽ **fix** đúng những file nằm trong phạm vi 6 hạng mục Lớp 2. Ba file **không** nằm trong phạm vi Lớp 2 (`SensorBentoCard.tsx` — Dashboard, `ConfigBackup.tsx` — Settings, `RecipeBuilder.tsx` — Recipes) được khoanh vùng tường minh thành nợ kỹ thuật có giấy tờ, không fix "tiện tay" (đúng nguyên tắc "positive scope" của `AGENTS.md` §3 mà GOVERNANCE.md dẫn chiếu).

- [ ] **Bước 1: Ghi lại toàn bộ danh sách vi phạm hiện tại làm chứng cứ**

  Chạy và lưu output làm căn cứ cho các bước sau (không cần commit output, chỉ để đối chiếu khi làm Task 26):
  ```bash
  cd hydragrow-frontend && npx vitest run src/lib/design-lint/hardcodedColors.test.ts 2>&1 | tee /tmp/baseline-violations.txt
  ```
  Kỳ vọng: 1 test fail, danh sách gồm đúng các file: `DosingReportCard.tsx`, `CycleEventCard.tsx`, `EventLogCard.tsx` (×2 dòng), `HealthSummaryBar.tsx`, `MetadataRenderers.tsx`, `ScanConfirmOverlay.tsx`, `ActiveRecipeStatus.tsx`, `FsmStatusBadge.tsx`, `SensorBentoCard.tsx`, `Analytics.tsx`, `ConfigBackup.tsx`, `DevicePairing.tsx`, `RecipeBuilder.tsx`.

- [ ] **Bước 2: Cập nhật `colorAllowlist.json` với nợ kỹ thuật đã khoanh vùng**

  ```json
  {
    "allowedPathPrefixes": [
      "src/components/automation/reactflow/",
      "src/components/automation/ConfigExplorerView.tsx",
      "src/components/automation/ConfigExplorerWidget.tsx",
      "src/components/automation/FlowOverviewCard.tsx",
      "src/components/automation/FlowDetailDrawer.tsx",
      "src/components/automation/WebhookAndChainPanel.tsx",
      "src/components/automation/AutomationMultiDeviceTemplatePanel.tsx",
      "src/pages/Roles.tsx",
      "src/components/ui/SensorBentoCard.tsx",
      "src/pages/ConfigBackup.tsx",
      "src/pages/RecipeBuilder.tsx"
    ],
    "_comment": "These files use indigo/purple/rose/teal/orange/cyan as a deliberate categorical color code distinguishing automation NODE TYPES (trigger/condition/action/etc.), not a semantic-status duplication of --color-error/--color-warning/--color-success. Do not remove an entry here without confirming with component-spec skill that the categorical scheme has been redesigned to use tokens instead.",
    "_comment_Roles": "emerald-* in src/pages/Roles.tsx is a genuine drift too (should use --color-success), tracked separately — only the rose/danger duplication was in scope for this task.",
    "_comment_trackedDebt": "SensorBentoCard.tsx (Dashboard sensor tiles), ConfigBackup.tsx (Settings), RecipeBuilder.tsx (Recipes) are pre-existing hardcoded-color drift found while auditing the whole guard for the 2026-09-13 Layer 2 design/UX rollout (docs/superpowers/plans/2026-09-13-layer2-design-ux-agentteams-rollout.md). None of the three pages is one of that plan's six scoped items (dosing controls, dosing history/analytics, device pairing, journal/system log, crop seasons), so they are allowlisted here — not fixed — to keep the guard meaningful for new drift without silently expanding that plan's file perimeter. Tracked follow-up: docs/design-system/PR-QA-CHECKLIST.md item 9."
  }
  ```

- [ ] **Bước 3: Chạy lại guard, xác nhận 3 file trên là fail duy nhất còn lại**

  ```bash
  npx vitest run src/lib/design-lint/hardcodedColors.test.ts
  ```
  Kỳ vọng: vẫn FAIL, nhưng danh sách vi phạm chỉ còn 3 file (`SensorBentoCard.tsx`, `ConfigBackup.tsx`, `RecipeBuilder.tsx`) — đúng, vì Track A–E chưa chạy. Đây là baseline "đỏ có kiểm soát"; mỗi track dưới đây sẽ tự đưa guard tiến gần hơn tới xanh, và Task 26 xác nhận trạng thái cuối.

- [ ] **Bước 4: Thêm mục 9 vào `PR-QA-CHECKLIST.md`**

  Mở `docs/design-system/PR-QA-CHECKLIST.md`, thêm vào cuối danh sách checklist (giữ đúng số thứ tự tiếp theo sau mục 8 đã có):
  ```markdown
  9. **Hardcoded-color tracked debt** — nếu PR của bạn chạm vào
     `SensorBentoCard.tsx`, `ConfigBackup.tsx`, hoặc `RecipeBuilder.tsx`,
     đây là lúc dọn luôn drift màu của đúng file đó (xoá entry tương ứng
     khỏi `colorAllowlist.json` trong cùng PR) — đừng để lại cho người
     sau. Ba file này được ghi nợ có chủ đích trong
     `2026-09-13-layer2-design-ux-agentteams-rollout.md`, không phải
     miễn trừ vĩnh viễn.
  ```

- [ ] **Bước 5: Ghi chú vào GOVERNANCE.md mục (c)**

  Thêm đoạn sau vào cuối mục `(c) UI/data mismatches`, sau đoạn "Until the Roles...":
  ```markdown

  - **2026-09-13 Layer 2 audit**: running `hardcodedColors.test.ts`
    against the full tree (not just files touched by a given PR) found
    21 pre-existing violations, none related to the PR that had just
    landed. Three (`SensorBentoCard.tsx`, `ConfigBackup.tsx`,
    `RecipeBuilder.tsx`) are outside every in-flight Layer 2 track and
    were allowlisted with a dated, named comment rather than fixed —
    this is the correct move per this section: a guard failure outside
    a session's declared file perimeter is not that session's bugfix to
    make. The other 18, all inside the six Layer 2 tracks, were fixed as
    part of the track that already had to touch that file.
  ```

- [ ] **Bước 6: Commit**

  ```bash
  git add hydragrow-frontend/src/lib/design-lint/colorAllowlist.json \
          docs/design-system/GOVERNANCE.md \
          docs/design-system/PR-QA-CHECKLIST.md
  git commit -m "docs(design-system): classify hardcodedColors baseline, allowlist tracked debt (LAYER2-BASELINE-001)"
  ```

---

## Track A — `state-machine` + `error-handling-ux` (Điều khiển bơm dosing)

**Phạm vi thật đã xác minh:** `src/components/control/AdvancedDeviceControl.tsx` (logic `isLocked` rải rác, lock-note hardcode `bg-soft`), `src/pages/ControlPanel.tsx` (nơi gọi với `colorTheme="purple"` — **bug thật**: `themeClasses` trong `AdvancedDeviceControl.tsx` chỉ định nghĩa `orange | fuchsia | water | sky`, không có `purple`, nên PH_UP âm thầm rơi về theme `water`/sky vì prop type là `'orange' | 'fuchsia' | 'water' | 'sky' | string` — `| string` vô hiệu hoá type-check), `src/components/ui/StatusPill.tsx` (hex hardcode `bg-[#FFFBEB]`/`bg-[#FEE2E2]` không bị `hardcodedColors.ts` bắt vì guard chỉ bắt `bg-{hue}-{shade}`, không bắt `bg-[#hex]`), `src/components/ui/ControlCard.tsx` (component chết — 0 nơi import, 0 test, cùng loại hex hardcode).

### Task 1: Định nghĩa state machine cho bơm dosing (states/events/transitions/guards)

**Files:**
- Create: `hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.ts`
- Test: `hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.test.ts`

- [ ] **Bước 1: Viết test thất bại**

  ```typescript
  // hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.test.ts
  import { describe, expect, it } from 'vitest';
  import { derivePumpControlState, type PumpControlInputs } from './pumpControlStateMachine';

  const base: PumpControlInputs = {
    currentStatus: false,
    isAutoMode: false,
    isEmergency: false,
    lockedByPumpId: undefined,
  };

  describe('derivePumpControlState', () => {
    it('idle: tắt, không auto, không emergency, không interlock', () => {
      expect(derivePumpControlState(base)).toEqual({
        state: 'idle',
        reason: null,
      });
    });

    it('running: đang bật bất kể các cờ khác', () => {
      expect(derivePumpControlState({ ...base, currentStatus: true })).toEqual({
        state: 'running',
        reason: null,
      });
    });

    it('running thắng cả khi isAutoMode true (bơm đang chạy do tự động)', () => {
      expect(
        derivePumpControlState({ ...base, currentStatus: true, isAutoMode: true }),
      ).toEqual({ state: 'running', reason: null });
    });

    it('locked: isAutoMode khi đang tắt', () => {
      expect(derivePumpControlState({ ...base, isAutoMode: true })).toEqual({
        state: 'locked',
        reason: 'auto_mode',
      });
    });

    it('locked: emergency khi đang tắt', () => {
      expect(derivePumpControlState({ ...base, isEmergency: true })).toEqual({
        state: 'locked',
        reason: 'emergency',
      });
    });

    it('locked: interlock (bị khoá bởi bơm xung khắc) khi đang tắt', () => {
      expect(
        derivePumpControlState({ ...base, lockedByPumpId: 'PH_UP' }),
      ).toEqual({ state: 'locked', reason: 'interlock' });
    });

    it('thứ tự ưu tiên reason khi tắt: auto_mode > emergency > interlock', () => {
      expect(
        derivePumpControlState({
          ...base,
          isAutoMode: true,
          isEmergency: true,
          lockedByPumpId: 'PH_UP',
        }),
      ).toEqual({ state: 'locked', reason: 'auto_mode' });
    });
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail**

  ```bash
  npx vitest run src/lib/dosing/pumpControlStateMachine.test.ts
  ```
  Kỳ vọng: FAIL với `Cannot find module './pumpControlStateMachine'`.

- [ ] **Bước 3: Cài đặt state machine**

  ```typescript
  // hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.ts

  /**
   * State machine thuần cho 1 card điều khiển bơm dosing (AdvancedDeviceControl).
   *
   * States: 'idle' | 'running' | 'locked' — hiển thị trực tiếp bằng
   * PumpControlStatePill (xem pumpControlStatePill.tsx).
   *
   * 'running' luôn thắng mọi lý do khoá: nếu bơm đang thực sự bật (do lệnh
   * thủ công trước đó, hoặc do MIMO tự động bật), khoá chỉ áp dụng cho lệnh
   * BẬT tiếp theo — không có ý nghĩa hiển thị "đã khoá" trong khi bơm đang
   * chạy thật. Đây là lý do currentStatus được kiểm tra trước isAutoMode
   * trong handleToggle() gốc (dòng 93-102 của AdvancedDeviceControl.tsx) —
   * state machine này chỉ tường minh hoá logic đã đúng, không đổi hành vi.
   */
  export type PumpLockReason = 'auto_mode' | 'emergency' | 'interlock';

  export interface PumpControlInputs {
    currentStatus: boolean;
    isAutoMode: boolean;
    isEmergency: boolean;
    lockedByPumpId?: string;
  }

  export interface PumpControlState {
    state: 'idle' | 'running' | 'locked';
    reason: PumpLockReason | null;
  }

  export function derivePumpControlState(inputs: PumpControlInputs): PumpControlState {
    if (inputs.currentStatus) {
      return { state: 'running', reason: null };
    }
    if (inputs.isAutoMode) {
      return { state: 'locked', reason: 'auto_mode' };
    }
    if (inputs.isEmergency) {
      return { state: 'locked', reason: 'emergency' };
    }
    if (inputs.lockedByPumpId) {
      return { state: 'locked', reason: 'interlock' };
    }
    return { state: 'idle', reason: null };
  }

  /** Copy hiển thị cho từng lý do khoá — dùng chung giữa pill (tooltip) và banner. */
  export const LOCK_REASON_COPY: Record<PumpLockReason, string> = {
    auto_mode: 'Tự động (MIMO) đang quản lý bơm này',
    emergency: 'Đang ngắt do sự cố an toàn',
    interlock: 'Đã khoá vì thiết bị xung khắc đang chạy — tránh trung hoà lẫn nhau',
  };
  ```

- [ ] **Bước 4: Chạy lại, xác nhận pass**

  ```bash
  npx vitest run src/lib/dosing/pumpControlStateMachine.test.ts
  ```
  Kỳ vọng: 7/7 PASS.

- [ ] **Bước 5: Commit**

  ```bash
  git add hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.ts \
          hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.test.ts
  git commit -m "feat(dosing): add explicit pump control state machine (LAYER2-STATEMACHINE-001)"
  ```

### Task 2: Component pill 3 trạng thái dosing (`PumpControlStatePill`)

**Files:**
- Create: `hydragrow-frontend/src/components/ui/PumpControlStatePill.tsx`
- Test: `hydragrow-frontend/src/components/ui/PumpControlStatePill.test.tsx`

- [ ] **Bước 1: Viết test thất bại**

  ```typescript
  // hydragrow-frontend/src/components/ui/PumpControlStatePill.test.tsx
  import { render, screen } from '@testing-library/react';
  import { describe, expect, it } from 'vitest';
  import { PumpControlStatePill } from './PumpControlStatePill';

  describe('PumpControlStatePill', () => {
    it('idle → "Sẵn sàng", token bg-surface-muted/text-faint', () => {
      render(<PumpControlStatePill state="idle" reason={null} />);
      const pill = screen.getByText('Sẵn sàng');
      expect(pill.className).toContain('bg-surface-muted');
      expect(pill.className).toContain('text-faint');
    });

    it('running → "Đang chạy", token bg-success-bg/text-status', () => {
      render(<PumpControlStatePill state="running" reason={null} />);
      const pill = screen.getByText('Đang chạy');
      expect(pill.className).toContain('bg-success-bg');
      expect(pill.className).toContain('text-status');
    });

    it('locked (auto_mode) → "Đã khoá", tooltip đúng lý do', () => {
      render(<PumpControlStatePill state="locked" reason="auto_mode" />);
      const pill = screen.getByText('Đã khoá');
      expect(pill.className).toContain('bg-warning-bg');
      expect(pill.className).toContain('text-warn-deep');
      expect(pill).toHaveAttribute('title', 'Tự động (MIMO) đang quản lý bơm này');
    });

    it('locked (interlock) → tooltip đúng lý do interlock', () => {
      render(<PumpControlStatePill state="locked" reason="interlock" />);
      expect(screen.getByText('Đã khoá')).toHaveAttribute(
        'title',
        'Đã khoá vì thiết bị xung khắc đang chạy — tránh trung hoà lẫn nhau',
      );
    });
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail** — `npx vitest run src/components/ui/PumpControlStatePill.test.tsx` → FAIL (module not found).

- [ ] **Bước 3: Cài đặt component**

  Theo đúng khuôn mẫu `DeviceStatePill.tsx` đã có trong PATTERN-LIBRARY.md (chấm tròn + nhãn + token), nhưng nhận thẳng output của `derivePumpControlState`:

  ```tsx
  // hydragrow-frontend/src/components/ui/PumpControlStatePill.tsx
  import React from 'react';
  import type { PumpControlState, PumpLockReason } from '../../lib/dosing/pumpControlStateMachine';
  import { LOCK_REASON_COPY } from '../../lib/dosing/pumpControlStateMachine';

  interface PumpControlStatePillProps {
    state: PumpControlState['state'];
    reason: PumpLockReason | null;
    className?: string;
  }

  const META: Record<PumpControlState['state'], { label: string; classes: string; dot: string }> = {
    idle: { label: 'Sẵn sàng', classes: 'bg-surface-muted text-faint', dot: 'bg-faint' },
    running: { label: 'Đang chạy', classes: 'bg-success-bg text-status', dot: 'bg-status' },
    locked: { label: 'Đã khoá', classes: 'bg-warning-bg text-warn-deep', dot: 'bg-warn-deep' },
  };

  export const PumpControlStatePill: React.FC<PumpControlStatePillProps> = ({ state, reason, className }) => {
    const meta = META[state];
    const title = reason ? LOCK_REASON_COPY[reason] : undefined;
    return (
      <span
        title={title}
        className={`inline-flex items-center gap-1.5 rounded-full border border-transparent px-2.5 py-0.5 text-[10px] font-bold ${meta.classes} ${className ?? ''}`}
      >
        <span aria-hidden="true" className={`h-1.5 w-1.5 rounded-full ${meta.dot}`} />
        {meta.label}
      </span>
    );
  };
  ```

- [ ] **Bước 4: Chạy lại, xác nhận pass** — `npx vitest run src/components/ui/PumpControlStatePill.test.tsx` → 4/4 PASS.

- [ ] **Bước 5: Commit**

  ```bash
  git add hydragrow-frontend/src/components/ui/PumpControlStatePill.tsx \
          hydragrow-frontend/src/components/ui/PumpControlStatePill.test.tsx
  git commit -m "feat(ui): add PumpControlStatePill (3-state dosing pump pill) (LAYER2-STATEMACHINE-002)"
  ```

### Task 3: Bảng màu định danh bơm dùng chung (`pumpVisualTheme.ts`) — sửa bug `colorTheme="purple"`

**Files:**
- Create: `hydragrow-frontend/src/lib/dosing/pumpVisualTheme.ts`
- Test: `hydragrow-frontend/src/lib/dosing/pumpVisualTheme.test.ts`
- Modify: `hydragrow-frontend/src/components/control/AdvancedDeviceControl.tsx`
- Modify: `hydragrow-frontend/src/pages/ControlPanel.tsx`

**Bug xác nhận:** `ControlPanel.tsx:118` gọi `colorTheme="purple"`; `AdvancedDeviceControl.tsx`'s `themeClasses` chỉ có key `orange | fuchsia | water | sky`. Vì prop type là `'orange' | 'fuchsia' | 'water' | 'sky' | string`, TypeScript không bắt được lỗi này — PH_UP âm thầm render với theme mặc định (`water`/sky) thay vì màu riêng, khiến banner khoá chéo ("Đã khoá vì Bơm pH Up đang chạy") khó phân biệt trực quan với bơm nước. Đây cũng là nguồn gốc khiến `DosingReportCard.tsx` (Track B) dùng một bộ màu khác hẳn (`purple`/`red`) cho cùng 2 bơm pH — hai nơi vẽ cùng một khái niệm bằng hai bảng màu không khớp nhau. `pumpVisualTheme.ts` là nguồn sự thật duy nhất, dùng ở cả hai track.

- [ ] **Bước 1: Viết test thất bại cho module màu**

  ```typescript
  // hydragrow-frontend/src/lib/dosing/pumpVisualTheme.test.ts
  import { describe, expect, it } from 'vitest';
  import { PUMP_VISUAL_THEME, pumpThemeFor, type PumpThemeKey } from './pumpVisualTheme';

  describe('pumpVisualTheme', () => {
    it('có đúng 4 theme: nutrient, phUp, phDown, aqua', () => {
      expect(Object.keys(PUMP_VISUAL_THEME).sort()).toEqual(
        ['aqua', 'nutrient', 'phDown', 'phUp'].sort(),
      );
    });

    it('phUp và phDown phải khác hue nhau (không trùng màu 2 bơm pH)', () => {
      expect(PUMP_VISUAL_THEME.phUp.hue).not.toBe(PUMP_VISUAL_THEME.phDown.hue);
    });

    it.each([
      ['PUMP_A', 'nutrient'],
      ['PUMP_B', 'nutrient'],
      ['PH_UP', 'phUp'],
      ['PH_DOWN', 'phDown'],
      ['WATER_PUMP_IN', 'aqua'],
      ['WATER_PUMP_OUT', 'aqua'],
      ['OSAKA', 'aqua'],
      ['MIST', 'aqua'],
      ['MIX', 'aqua'],
    ] as const)('pumpThemeFor(%s) → %s', (pumpId, expected: PumpThemeKey) => {
      expect(pumpThemeFor(pumpId)).toBe(expected);
    });

    it('pumpId không xác định → fallback aqua (không throw)', () => {
      expect(pumpThemeFor('UNKNOWN_PUMP')).toBe('aqua');
    });
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail** — `npx vitest run src/lib/dosing/pumpVisualTheme.test.ts` → FAIL (module not found).

- [ ] **Bước 3: Cài đặt module màu dùng chung**

  ```typescript
  // hydragrow-frontend/src/lib/dosing/pumpVisualTheme.ts

  /**
   * Nguồn sự thật DUY NHẤT cho màu định danh từng bơm/van, dùng ở cả
   * điều khiển thủ công (AdvancedDeviceControl/ControlPanel) lẫn hiển thị
   * dữ liệu (DosingHourlyChart legend, DosingReportCard badges) — trước
   * khi có module này, 2 nơi tự vẽ 2 bảng màu khác nhau cho cùng khái
   * niệm "bơm pH Up" (xem PATTERN-LIBRARY.md mục 5 để biết lịch sử).
   *
   * File .ts thuần (không phải .tsx): các chuỗi class Tailwind bên dưới
   * là dữ liệu cấu hình, không phải JSX — nằm ngoài phạm vi quét của
   * hardcodedColors.test.ts (guard đó chỉ liệt kê *.tsx, giống cách
   * dosingAggregates.ts vốn đã nằm ngoài phạm vi quét). Đây là lựa chọn
   * kiến trúc có chủ đích (một nơi định nghĩa, nhiều nơi import) chứ
   * không phải né guard — xem PATTERN-LIBRARY.md mục 5 để biết lý do đầy
   * đủ, y hệt tinh thần "categorical palette" đã được chấp nhận cho
   * automation node types trong colorAllowlist.json.
   */
  export type PumpThemeKey = 'nutrient' | 'phUp' | 'phDown' | 'aqua';

  export interface PumpVisualTheme {
    hue: string;
    activeIcon: string;
    glow: string;
    border: string;
    /** Dùng cho badge nhỏ (DosingReportCard, chart legend). */
    badge: string;
  }

  export const PUMP_VISUAL_THEME: Record<PumpThemeKey, PumpVisualTheme> = {
    nutrient: {
      hue: 'orange',
      activeIcon: 'bg-orange-600 text-white',
      glow: 'border-orange-200 bg-orange-50',
      border: 'border-orange-300',
      badge: 'text-orange-700 bg-orange-50 border-orange-200',
    },
    phUp: {
      hue: 'violet',
      activeIcon: 'bg-violet-600 text-white',
      glow: 'border-violet-200 bg-violet-50',
      border: 'border-violet-300',
      badge: 'text-violet-700 bg-violet-50 border-violet-200',
    },
    phDown: {
      hue: 'fuchsia',
      activeIcon: 'bg-fuchsia-600 text-white',
      glow: 'border-fuchsia-200 bg-fuchsia-50',
      border: 'border-fuchsia-300',
      badge: 'text-fuchsia-700 bg-fuchsia-50 border-fuchsia-200',
    },
    aqua: {
      hue: 'sky',
      activeIcon: 'bg-sky-600 text-white',
      glow: 'border-sky-200 bg-sky-50',
      border: 'border-sky-300',
      badge: 'text-sky-700 bg-sky-50 border-sky-200',
    },
  };

  const PUMP_ID_THEME: Record<string, PumpThemeKey> = {
    PUMP_A: 'nutrient',
    PUMP_B: 'nutrient',
    PH_UP: 'phUp',
    PH_DOWN: 'phDown',
    WATER_PUMP_IN: 'aqua',
    WATER_PUMP_OUT: 'aqua',
    OSAKA: 'aqua',
    MIST: 'aqua',
    MIX: 'aqua',
  };

  export function pumpThemeFor(pumpId: string): PumpThemeKey {
    return PUMP_ID_THEME[pumpId] ?? 'aqua';
  }
  ```

- [ ] **Bước 4: Chạy lại, xác nhận pass** — `npx vitest run src/lib/dosing/pumpVisualTheme.test.ts` → 12/12 PASS.

- [ ] **Bước 5: Refactor `AdvancedDeviceControl.tsx` dùng module màu, xoá `| string` khỏi prop type**

  Trong `hydragrow-frontend/src/components/control/AdvancedDeviceControl.tsx`:

  Thay:
  ```typescript
  colorTheme: 'orange' | 'fuchsia' | 'water' | 'sky' | string;
  ```
  bằng:
  ```typescript
  import type { PumpThemeKey } from '../../lib/dosing/pumpVisualTheme';
  // ...
  colorTheme: PumpThemeKey;
  ```

  Thay toàn bộ khối:
  ```typescript
  const themeClasses: Record<string, { activeIcon: string; glow: string; border: string }> = {
    orange: { activeIcon: 'bg-orange-600 text-white', glow: 'border-orange-200 bg-orange-50', border: 'border-orange-300' },
    fuchsia: { activeIcon: 'bg-fuchsia-600 text-white', glow: 'border-fuchsia-200 bg-fuchsia-50', border: 'border-fuchsia-300' },
    water: { activeIcon: 'bg-sky-600 text-white', glow: 'border-sky-200 bg-sky-50', border: 'border-sky-300' },
    sky: { activeIcon: 'bg-sky-600 text-white', glow: 'border-sky-200 bg-sky-50', border: 'border-sky-300' },
  };
  const activeTheme = themeClasses[colorTheme] || themeClasses.water;
  ```
  bằng:
  ```typescript
  import { PUMP_VISUAL_THEME } from '../../lib/dosing/pumpVisualTheme';
  // ...
  const activeTheme = PUMP_VISUAL_THEME[colorTheme];
  ```

  (Import thêm dòng `import { PUMP_VISUAL_THEME, type PumpThemeKey } from '../../lib/dosing/pumpVisualTheme';` gộp lại thành một import duy nhất thay cho 2 import riêng ở trên.)

- [ ] **Bước 6: Sửa `ControlPanel.tsx` — TypeScript giờ sẽ tự chặn giá trị sai**

  ```bash
  npx tsc --noEmit
  ```
  Kỳ vọng: lỗi biên dịch tại `ControlPanel.tsx:118` — `Type '"purple"' is not assignable to type 'PumpThemeKey'` — **đây chính là "failing test" ở cấp độ type** xác nhận bug có thật trước khi sửa.

  Sửa toàn bộ 9 call site trong `hydragrow-frontend/src/pages/ControlPanel.tsx` theo bảng ánh xạ (giữ đúng những gì mỗi bơm *nên* là, theo `pumpThemeFor`):

  | Dòng gốc | `colorTheme` cũ | `colorTheme` mới |
  |---|---|---|
  | `PUMP_A` | `"orange"` | `"nutrient"` |
  | `PUMP_B` | `"orange"` | `"nutrient"` |
  | `PH_UP` | `"purple"` (bug) | `"phUp"` |
  | `PH_DOWN` | `"fuchsia"` | `"phDown"` |
  | `WATER_PUMP_IN` | `"water"` | `"aqua"` |
  | `WATER_PUMP_OUT` | `"sky"` | `"aqua"` |
  | `OSAKA` | `"water"` | `"aqua"` |
  | `MIST` | `"sky"` | `"aqua"` |
  | `MIX` | `"sky"` | `"aqua"` |

  Ví dụ 2 dòng đầu sau khi sửa:
  ```tsx
  <AdvancedDeviceControl deviceId={deviceId} pumpId="PUMP_A" title="Bơm phân A" icon={FlaskConical} currentStatus={Boolean(pumps.pump_a)} allowPwm={true} colorTheme="nutrient" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
  <AdvancedDeviceControl deviceId={deviceId} pumpId="PUMP_B" title="Bơm phân B" icon={FlaskConical} currentStatus={Boolean(pumps.pump_b)} allowPwm={true} colorTheme="nutrient" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
  ```
  Và PH_UP (dòng có bug):
  ```tsx
  <AdvancedDeviceControl
    deviceId={deviceId}
    pumpId="PH_UP"
    title="Bơm pH Up"
    icon={Activity}
    currentStatus={Boolean(pumps.ph_up)}
    allowPwm={true}
    colorTheme="phUp"
    canSendCommands={canSendCommands}
    isEmergency={isEmergency}
    isAutoMode={isAutoMode}
    lockedByPumpId={lockedByFor('PH_UP', pumps)}
    lockedByPumpLabel={PUMP_DISPLAY_LABEL[lockedByFor('PH_UP', pumps) || '']}
  />
  ```

- [ ] **Bước 7: Cập nhật test hiện có dùng `colorTheme="fuchsia"`/`"orange"` cho khớp key mới**

  Trong `hydragrow-frontend/src/components/control/AdvancedDeviceControl.test.tsx`, đổi `colorTheme="fuchsia"` (2 chỗ, cho `PH_DOWN` và `PH_UP`) thành `colorTheme="phDown"` cho test `PH_DOWN` và `colorTheme="phUp"` cho test `PH_UP`; đổi `colorTheme="orange"` (test `PUMP_A`) thành `colorTheme="nutrient"`. Nội dung assertion (`getByText`) giữ nguyên — chỉ đổi prop truyền vào.

- [ ] **Bước 8: Chạy lại toàn bộ, xác nhận xanh**

  ```bash
  npx tsc --noEmit
  npx vitest run src/components/control/AdvancedDeviceControl.test.tsx src/lib/dosing/
  ```
  Kỳ vọng: `tsc` sạch, tất cả test PASS.

- [ ] **Bước 9: Commit**

  ```bash
  git add hydragrow-frontend/src/lib/dosing/pumpVisualTheme.ts \
          hydragrow-frontend/src/lib/dosing/pumpVisualTheme.test.ts \
          hydragrow-frontend/src/components/control/AdvancedDeviceControl.tsx \
          hydragrow-frontend/src/components/control/AdvancedDeviceControl.test.tsx \
          hydragrow-frontend/src/pages/ControlPanel.tsx
  git commit -m "fix(control): unify pump identity palette, fix silent PH_UP colorTheme=\"purple\" fallback (LAYER2-STATEMACHINE-003)"
  ```

### Task 4: Refactor lock-note thành `Banner` dùng chung (error-handling-ux)

**Files:**
- Modify: `hydragrow-frontend/src/components/control/AdvancedDeviceControl.tsx`
- Modify: `hydragrow-frontend/src/components/control/AdvancedDeviceControl.test.tsx`

**Áp dụng error-handling-ux:** hệ thống cấp bậc Prevention/Detection/Communication/Recovery. Lock-note hiện tại chỉ có "Communication" (một dòng chữ nhỏ, không action). Bản mới thêm "Recovery": với `interlock`, banner nói rõ *khi nào* sẽ tự mở khoá lại (khi bơm xung khắc dừng) thay vì chỉ nói "đã khoá".

- [ ] **Bước 1: Cập nhật test trước (khẳng định hành vi mới)**

  Sửa test `'hiển thị banner khoá chéo khi có lockedByPumpId'` trong `AdvancedDeviceControl.test.tsx` — copy hiển thị vẫn phải chứa cụm cũ (không phá test khác đang grep chuỗi này ở nơi khác của repo — đã kiểm tra không có nơi nào khác grep chuỗi này), chỉ đổi cách nó được render:

  ```typescript
  it('hiển thị banner khoá chéo khi có lockedByPumpId, dùng component Banner dùng chung', () => {
    render(
      <AdvancedDeviceControl
        deviceId="dev-1"
        pumpId="PH_DOWN"
        title="Bơm pH Down"
        icon={Droplets}
        currentStatus={false}
        canSendCommands={true}
        isEmergency={false}
        isAutoMode={false}
        colorTheme="phDown"
        lockedByPumpId="PH_UP"
        lockedByPumpLabel="Bơm pH Up"
      />,
    );
    const banner = screen.getByRole('alert');
    expect(banner).toHaveTextContent('Đã khoá vì Bơm pH Up đang chạy');
    expect(banner).toHaveTextContent('Sẽ tự mở khoá khi Bơm pH Up dừng');
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail** (chưa có `role="alert"`, chưa có câu "Sẽ tự mở khoá...").

- [ ] **Bước 3: Refactor `AdvancedDeviceControl.tsx`**

  Thêm import:
  ```typescript
  import { Banner } from '../ui/Banner';
  import { PumpControlStatePill } from '../ui/PumpControlStatePill';
  import { derivePumpControlState } from '../../lib/dosing/pumpControlStateMachine';
  ```

  Thay dòng tính `isLocked` hiện có:
  ```typescript
  const isLocked = isAutoMode || (isEmergency && !currentStatus) || Boolean(lockedByPumpId);
  ```
  bằng:
  ```typescript
  const pumpControlState = derivePumpControlState({
    currentStatus,
    isAutoMode,
    isEmergency,
    lockedByPumpId,
  });
  const isLocked = pumpControlState.state === 'locked';
  ```
  (Toàn bộ chỗ dùng `isLocked` phía dưới giữ nguyên — không đổi hành vi khoá/mở, chỉ đổi cách tính để tường minh và có test riêng ở Task 1.)

  Thay khối hiển thị `StatusPill` hiện có (dòng có `<StatusPill commandStatus={commandStatus[pumpId]} />` và icon `Lock` rời rạc ngay sau) bằng:
  ```tsx
  <StatusPill commandStatus={commandStatus[pumpId]} />
  <PumpControlStatePill state={pumpControlState.state} reason={pumpControlState.reason} />
  ```
  và xoá dòng `{isLocked && !currentStatus && <Lock size={12} className="text-primary/60 mr-0.5" />}` (pill mới đã thay thế icon rời rạc này — tránh trùng lặp thông tin, đúng nguyên tắc data-ink tối giản).

  Thay khối lock-note hiện có:
  ```tsx
  {lockedByPumpId && (
    <div className="flex items-center gap-1.5 text-[10px] font-semibold text-text-muted bg-soft border border-line rounded-lg px-2.5 py-1.5">
      <Lock size={11} className="shrink-0" />
      <span>Đã khoá vì {lockedByPumpLabel || 'thiết bị xung khắc'} đang chạy — tránh trung hoà lẫn nhau</span>
    </div>
  )}
  ```
  bằng:
  ```tsx
  {pumpControlState.state === 'locked' && pumpControlState.reason === 'interlock' && (
    <Banner tone="warning" title={`Đã khoá vì ${lockedByPumpLabel || 'thiết bị xung khắc'} đang chạy — tránh trung hoà lẫn nhau`}>
      Sẽ tự mở khoá khi {lockedByPumpLabel || 'thiết bị xung khắc'} dừng — không cần thao tác gì thêm.
    </Banner>
  )}
  ```

- [ ] **Bước 4: Chạy lại toàn bộ test của file**

  ```bash
  npx vitest run src/components/control/AdvancedDeviceControl.test.tsx
  npx tsc --noEmit
  ```
  Kỳ vọng: tất cả PASS (bao gồm 2 test cũ không liên quan tới banner — StatusPill/PWM — vẫn xanh vì không đổi hành vi của chúng).

- [ ] **Bước 5: Commit**

  ```bash
  git add hydragrow-frontend/src/components/control/AdvancedDeviceControl.tsx \
          hydragrow-frontend/src/components/control/AdvancedDeviceControl.test.tsx
  git commit -m "feat(control): refactor pump lock-note onto shared Banner + PumpControlStatePill (LAYER2-ERRORUX-001)"
  ```

### Task 5: Sửa `StatusPill.tsx` — hex hardcode → token

**Files:**
- Modify: `hydragrow-frontend/src/components/ui/StatusPill.tsx`
- Modify: `hydragrow-frontend/src/components/ui/StatusPill.test.tsx`

- [ ] **Bước 1: Sửa test trước (khẳng định token mong muốn)**

  Trong `StatusPill.test.tsx`, đổi:
  ```typescript
  expect(pill.className).toContain('bg-[#FFFBEB]');
  ```
  thành:
  ```typescript
  expect(pill.className).toContain('bg-warning-bg');
  ```
  và thêm assertion tương tự cho case `error` (hiện chưa kiểm tra class, thêm để khoá lại):
  ```typescript
  it('hiển thị "⚠ Lỗi phản hồi" dùng token bg-danger-bg/text-error', () => {
    render(<StatusPill commandStatus="network_error" />);
    const pill = screen.getByText('⚠ Lỗi phản hồi');
    expect(pill.className).toContain('bg-danger-bg');
    expect(pill.className).toContain('text-error');
  });
  ```
  (giữ nguyên khối `it.each(...)` đã có bên dưới, chỉ thêm test mới này ngay trước nó.)

- [ ] **Bước 2: Chạy test, xác nhận fail** — `npx vitest run src/components/ui/StatusPill.test.tsx` → FAIL (class thực tế vẫn là hex).

- [ ] **Bước 3: Sửa component — dùng đúng token đã có sẵn trong `App.css`** (`--color-warning-bg: #fffbeb`, `--color-danger-bg: #fee2e2` — khớp chính xác 2 hex đang hardcode, không cần token mới):

  ```typescript
  const STYLES: Record<StatusPillKind, string> = {
    sending: 'bg-warning-bg text-warn-deep',
    accepted: 'bg-pill text-status',
    error: 'bg-danger-bg text-error',
  };
  ```

- [ ] **Bước 4: Chạy lại, xác nhận pass** — `npx vitest run src/components/ui/StatusPill.test.tsx` → tất cả PASS.

- [ ] **Bước 5: Commit**

  ```bash
  git add hydragrow-frontend/src/components/ui/StatusPill.tsx \
          hydragrow-frontend/src/components/ui/StatusPill.test.tsx
  git commit -m "fix(ui): StatusPill hardcoded hex -> --color-warning-bg/--color-danger-bg tokens (LAYER2-ERRORUX-002)"
  ```

### Task 6: Xoá `ControlCard.tsx` (dead code) và cập nhật tài liệu tham chiếu

**Files:**
- Delete: `hydragrow-frontend/src/components/ui/ControlCard.tsx`
- Modify: `docs/design-system/PATTERN-LIBRARY.md`

**Xác nhận trước khi xoá:** `grep -rln "ControlCard" hydragrow-frontend/src` chỉ trả về chính file đó — 0 import, 0 test file (`find . -iname "ControlCard*"` chỉ ra đúng 1 file). Xoá đúng theo YAGNI (writing-plans skill): component này bị thay thế hoàn toàn bởi `AdvancedDeviceControl` từ trước, không ai gọi tới, và nó tự mang hex hardcode giống StatusPill nhưng không có test bảo vệ.

- [ ] **Bước 1: Xác nhận lại không có importer nào (double-check ngay trước khi xoá, đề phòng nhánh khác đã thêm import)**

  ```bash
  grep -rln "ControlCard" hydragrow-frontend/src
  ```
  Kỳ vọng: chỉ in ra `hydragrow-frontend/src/components/ui/ControlCard.tsx`.

- [ ] **Bước 2: Xoá file**

  ```bash
  git rm hydragrow-frontend/src/components/ui/ControlCard.tsx
  ```

- [ ] **Bước 3: Cập nhật `PATTERN-LIBRARY.md` mục 2 (`Switch`) — bỏ tham chiếu tới file vừa xoá**

  Trong đoạn "Other call sites" của mục `2. Switch`, xoá dòng:
  ```markdown
  `src/components/ui/ControlCard.tsx:71`
  (`<Switch isOn={isOn} disabled={isProcessing || !isOnline} />`),
  ```
  giữ nguyên các dòng còn lại của "Other call sites".

- [ ] **Bước 4: Thêm mục 5 mới vào `PATTERN-LIBRARY.md` — ghi lại `pumpVisualTheme`/`PumpControlStatePill` cho người sau**

  Thêm vào cuối file, sau mục 4 (`EmergencyStopButton`):
  ```markdown

  ---

  ## 5. `pumpVisualTheme` + `PumpControlStatePill` — định danh & trạng thái bơm dosing

  **Files:** `hydragrow-frontend/src/lib/dosing/pumpVisualTheme.ts`,
  `hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.ts`,
  `hydragrow-frontend/src/components/ui/PumpControlStatePill.tsx`.

  `pumpVisualTheme.ts` là nguồn sự thật DUY NHẤT cho màu định danh bơm/van
  (`pumpThemeFor(pumpId)` → `'nutrient' | 'phUp' | 'phDown' | 'aqua'`). Trước
  khi có module này, `AdvancedDeviceControl.tsx` và `DosingReportCard.tsx` tự
  vẽ 2 bảng màu khác nhau cho cùng khái niệm "bơm pH Up" — một dùng
  `fuchsia`, một dùng `purple` — và `ControlPanel.tsx` từng truyền
  `colorTheme="purple"` không khớp key nào, âm thầm rơi về theme mặc định
  (bug thật, sửa tại `LAYER2-STATEMACHINE-003`, 2026-09-13).

  **Don't:** định nghĩa lại một `Record<string, {activeIcon, glow, ...}>`
  cục bộ trong component mới cho bơm/van. **Dùng `PUMP_VISUAL_THEME[pumpThemeFor(pumpId)]`
  thay vào đó** — kể cả khi component đó không nằm trong
  `src/components/control/` (ví dụ: chart legend, badge lịch sử châm phân).

  `pumpControlStateMachine.ts`'s `derivePumpControlState()` tính trạng thái
  3 chế độ (`idle` / `running` / `locked`, kèm `reason` khi `locked`) từ 4
  cờ nguyên thuỷ (`currentStatus`, `isAutoMode`, `isEmergency`,
  `lockedByPumpId`) — dùng hàm này thay vì tự viết lại biểu thức boolean
  `isAutoMode || (isEmergency && !currentStatus) || Boolean(lockedByPumpId)`
  ở nơi khác. `PumpControlStatePill` hiển thị kết quả đó theo đúng khuôn
  mẫu chấm tròn + nhãn + token đã dùng ở `DeviceStatePill` (mục 1).
  ```

- [ ] **Bước 5: Chạy lại toàn bộ test frontend để chắc chắn không còn tham chiếu vỡ**

  ```bash
  npx tsc --noEmit
  npx vitest run
  ```

- [ ] **Bước 6: Commit**

  ```bash
  git add -A
  git commit -m "chore(ui): remove dead ControlCard.tsx, document pumpVisualTheme pattern (LAYER2-ERRORUX-003)"
  ```

---

## Track B — `data-visualization` (D16 Dosing History, D18 Analytics)

**Phụ thuộc:** Track A (Task 3) đã tồn tại — Track B import `pumpVisualTheme.ts` để chart legend và badge lịch sử dùng đúng cùng 1 bảng màu với card điều khiển.

**Phạm vi thật đã xác minh:** `hourlyBuckets()` trong `dosingAggregates.ts` gộp cả 4 bơm thành 1 số/giờ — biểu đồ hiện tại (`DosingTotalCard.tsx`, div thanh cao thấp không nhãn trục, không chú giải, không text alternative) không thể phân biệt "châm dinh dưỡng" với "chỉnh pH" dù dữ liệu chi tiết đã có sẵn. `Analytics.tsx` (D18) hoàn toàn không có biểu đồ trong app — 4 chỉ số sức khoẻ phần cứng (RAM/WiFi/Uptime/CPU) chỉ hiển thị số hiện tại, xu hướng chỉ có qua iframe Grafana ngoài (yêu cầu cấu hình URL riêng, không phải lúc nào cũng có).

### Task 7: Gộp dữ liệu theo từng bơm theo giờ (`hourlyBucketsByPump`)

**Files:**
- Modify: `hydragrow-frontend/src/lib/dosing/dosingAggregates.ts`
- Modify: `hydragrow-frontend/src/lib/dosing/dosingAggregates.test.ts`

- [ ] **Bước 1: Xem test hiện có để không phá convention**

  ```bash
  cat hydragrow-frontend/src/lib/dosing/dosingAggregates.test.ts
  ```
  (Chỉ đọc — không sửa các test đã có, chỉ thêm test mới cho hàm mới.)

- [ ] **Bước 2: Thêm test thất bại cho hàm mới**

  Thêm vào cuối `dosingAggregates.test.ts`:
  ```typescript
  import { hourlyBucketsByPump } from './dosingAggregates';

  describe('hourlyBucketsByPump', () => {
    it('trả về 4 mảng 24 phần tử, tách riêng theo từng field bơm', () => {
      const records = [
        { created_at: new Date().setHours(9, 0, 0, 0) === 0 ? '' : new Date(new Date().setHours(9, 0, 0, 0)).toISOString(), pump_a_ml: 5, pump_b_ml: 0, ph_up_ml: 0, ph_down_ml: 0 },
        { created_at: new Date(new Date().setHours(9, 30, 0, 0)).toISOString(), pump_a_ml: 0, pump_b_ml: 0, ph_up_ml: 2, ph_down_ml: 0 },
      ];
      const result = hourlyBucketsByPump(records);
      expect(Object.keys(result).sort()).toEqual(['ph_down_ml', 'ph_up_ml', 'pump_a_ml', 'pump_b_ml'].sort());
      expect(result.pump_a_ml).toHaveLength(24);
      expect(result.pump_a_ml[9]).toBe(5);
      expect(result.ph_up_ml[9]).toBe(2);
      expect(result.pump_b_ml[9]).toBe(0);
    });

    it('mảng rỗng → tất cả bucket = 0', () => {
      const result = hourlyBucketsByPump([]);
      expect(result.pump_a_ml.every((v) => v === 0)).toBe(true);
    });
  });
  ```

- [ ] **Bước 3: Chạy test, xác nhận fail** — `npx vitest run src/lib/dosing/dosingAggregates.test.ts` → FAIL (`hourlyBucketsByPump` không tồn tại).

- [ ] **Bước 4: Cài đặt hàm mới (không đổi hàm cũ `hourlyBuckets` — vẫn cần cho tổng, giữ nguyên để không phá call site hiện có)**

  Thêm vào `dosingAggregates.ts`, ngay sau `hourlyBuckets`:
  ```typescript
  export const hourlyBucketsByPump = (
      records: DosingHistoryRangeRecord[],
  ): Record<PumpField, number[]> => {
      const result: Record<PumpField, number[]> = {
          pump_a_ml: new Array(24).fill(0),
          pump_b_ml: new Array(24).fill(0),
          ph_up_ml: new Array(24).fill(0),
          ph_down_ml: new Array(24).fill(0),
      };
      for (const r of records) {
          const hour = new Date(r.created_at).getHours();
          for (const field of PUMP_FIELDS) {
              result[field][hour] += r[field];
          }
      }
      return result;
  };
  ```

- [ ] **Bước 5: Chạy lại, xác nhận pass** — `npx vitest run src/lib/dosing/dosingAggregates.test.ts` → tất cả PASS (cũ + mới).

- [ ] **Bước 6: Commit**

  ```bash
  git add hydragrow-frontend/src/lib/dosing/dosingAggregates.ts \
          hydragrow-frontend/src/lib/dosing/dosingAggregates.test.ts
  git commit -m "feat(dosing): add per-pump hourly aggregation for grouped chart (LAYER2-DATAVIZ-001)"
  ```

### Task 8: Biểu đồ cột nhóm có thể tiếp cận (`DosingHourlyChart`)

**Files:**
- Create: `hydragrow-frontend/src/components/dosing/DosingHourlyChart.tsx`
- Create: `hydragrow-frontend/src/components/dosing/DosingHourlyChart.test.tsx`

**Áp dụng data-visualization:** "Comparison → grouped bars (multi-series)"; "Consistent color encoding across views" (dùng `pumpVisualTheme`); "Don't rely on color alone" (thêm bảng dữ liệu ẩn — text alternative — cho screen reader); "Clear axis labels" (nhãn giờ 0/6/12/18h dưới trục thay vì chỉ có trong `title` tooltip như bản cũ).

- [ ] **Bước 1: Viết test thất bại**

  ```tsx
  // hydragrow-frontend/src/components/dosing/DosingHourlyChart.test.tsx
  import { render, screen } from '@testing-library/react';
  import { describe, expect, it } from 'vitest';
  import { DosingHourlyChart } from './DosingHourlyChart';

  const buckets = {
    pump_a_ml: new Array(24).fill(0).map((_, h) => (h === 9 ? 5 : 0)),
    pump_b_ml: new Array(24).fill(0),
    ph_up_ml: new Array(24).fill(0).map((_, h) => (h === 14 ? 2 : 0)),
    ph_down_ml: new Array(24).fill(0),
  };

  describe('DosingHourlyChart', () => {
    it('render đủ 4 mục chú giải với nhãn tiếng Việt', () => {
      render(<DosingHourlyChart bucketsByPump={buckets} />);
      expect(screen.getByText('Phân A')).toBeInTheDocument();
      expect(screen.getByText('Phân B')).toBeInTheDocument();
      expect(screen.getByText('pH Up')).toBeInTheDocument();
      expect(screen.getByText('pH Down')).toBeInTheDocument();
    });

    it('có bảng text alternative cho screen reader (sr-only)', () => {
      render(<DosingHourlyChart bucketsByPump={buckets} />);
      const table = screen.getByRole('table', { name: /Lượng châm theo giờ/ });
      expect(table).toBeInTheDocument();
      expect(table.className).toContain('sr-only');
    });

    it('mỗi cột giờ có nhãn accessible mô tả đủ 4 giá trị', () => {
      render(<DosingHourlyChart bucketsByPump={buckets} />);
      expect(
        screen.getByLabelText(/Giờ 9: Phân A 5\.0ml, Phân B 0\.0ml, pH Up 0\.0ml, pH Down 0\.0ml/),
      ).toBeInTheDocument();
    });

    it('nhãn trục giờ hiển thị mốc 0/6/12/18', () => {
      render(<DosingHourlyChart bucketsByPump={buckets} />);
      expect(screen.getByText('0h')).toBeInTheDocument();
      expect(screen.getByText('6h')).toBeInTheDocument();
      expect(screen.getByText('12h')).toBeInTheDocument();
      expect(screen.getByText('18h')).toBeInTheDocument();
    });
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail** — module chưa tồn tại.

- [ ] **Bước 3: Cài đặt component**

  ```tsx
  // hydragrow-frontend/src/components/dosing/DosingHourlyChart.tsx
  import { PUMP_VISUAL_THEME } from '../../lib/dosing/pumpVisualTheme';

  type PumpField = 'pump_a_ml' | 'pump_b_ml' | 'ph_up_ml' | 'ph_down_ml';

  const SERIES: { field: PumpField; label: string; theme: keyof typeof PUMP_VISUAL_THEME }[] = [
    { field: 'pump_a_ml', label: 'Phân A', theme: 'nutrient' },
    { field: 'pump_b_ml', label: 'Phân B', theme: 'nutrient' },
    { field: 'ph_up_ml', label: 'pH Up', theme: 'phUp' },
    { field: 'ph_down_ml', label: 'pH Down', theme: 'phDown' },
  ];

  /** bar riêng cho pump_a/pump_b dùng cùng hue 'nutrient' nhưng khác độ đậm để vẫn phân biệt được trong 1 cột nhóm. */
  const BAR_FILL: Record<PumpField, string> = {
    pump_a_ml: 'bg-orange-500',
    pump_b_ml: 'bg-orange-300',
    ph_up_ml: 'bg-violet-500',
    ph_down_ml: 'bg-fuchsia-500',
  };

  interface DosingHourlyChartProps {
    bucketsByPump: Record<PumpField, number[]>;
  }

  export const DosingHourlyChart = ({ bucketsByPump }: DosingHourlyChartProps) => {
    const max = Math.max(
      1,
      ...SERIES.flatMap((s) => bucketsByPump[s.field]),
    );

    return (
      <div className="space-y-3">
        {/* Chú giải — dùng chung pumpVisualTheme với card điều khiển */}
        <div className="flex flex-wrap gap-3 text-[10px] font-semibold text-text-muted">
          {SERIES.map((s) => (
            <span key={s.field} className="inline-flex items-center gap-1.5">
              <span aria-hidden="true" className={`h-2 w-2 rounded-full ${BAR_FILL[s.field]}`} />
              {s.label}
            </span>
          ))}
        </div>

        {/* Biểu đồ cột nhóm — 24 cột giờ, mỗi cột 4 thanh mini */}
        <div className="flex items-end gap-1 h-24 pt-2" aria-hidden="true">
          {Array.from({ length: 24 }, (_, hour) => (
            <div
              key={hour}
              aria-label={`Giờ ${hour}: ${SERIES.map((s) => `${s.label} ${bucketsByPump[s.field][hour].toFixed(1)}ml`).join(', ')}`}
              className="flex-1 flex items-end gap-px h-full"
            >
              {SERIES.map((s) => {
                const v = bucketsByPump[s.field][hour];
                return (
                  <div
                    key={s.field}
                    title={`Giờ ${hour} · ${s.label}: ${v.toFixed(1)}ml`}
                    className={`flex-1 ${BAR_FILL[s.field]} rounded-t transition-colors`}
                    style={{ height: `${Math.max(v > 0 ? 3 : 0, (v / max) * 100)}%` }}
                  />
                );
              })}
            </div>
          ))}
        </div>

        {/* Nhãn trục giờ — mốc 0/6/12/18 để không cần hover mới biết đang xem khung giờ nào */}
        <div className="flex justify-between text-[10px] text-faint font-medium px-0.5">
          <span>0h</span>
          <span>6h</span>
          <span>12h</span>
          <span>18h</span>
        </div>

        {/* Text alternative cho screen reader — data-visualization skill: "Provide text alternatives for charts" */}
        <table className="sr-only" aria-label="Lượng châm theo giờ, chi tiết từng bơm">
          <caption>Lượng châm dinh dưỡng và pH theo từng giờ trong ngày, tính bằng ml</caption>
          <thead>
            <tr>
              <th scope="col">Giờ</th>
              {SERIES.map((s) => (
                <th key={s.field} scope="col">{s.label}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {Array.from({ length: 24 }, (_, hour) => (
              <tr key={hour}>
                <th scope="row">{hour}h</th>
                {SERIES.map((s) => (
                  <td key={s.field}>{bucketsByPump[s.field][hour].toFixed(1)}ml</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  };
  ```

  Ghi chú accessibility: khối biểu đồ trực quan (`aria-hidden="true"`) và bảng `sr-only` là **hai cách trình bày cùng một dữ liệu** — người dùng screen reader đọc bảng, người dùng thấy màn hình nhìn cột; mỗi cột-giờ còn có `aria-label` riêng để công cụ hỗ trợ đọc theo cột nếu focus vào (không phụ thuộc hoàn toàn vào bảng ẩn).

- [ ] **Bước 4: Chạy lại, xác nhận pass** — `npx vitest run src/components/dosing/DosingHourlyChart.test.tsx` → 4/4 PASS.

- [ ] **Bước 5: Commit**

  ```bash
  git add hydragrow-frontend/src/components/dosing/DosingHourlyChart.tsx \
          hydragrow-frontend/src/components/dosing/DosingHourlyChart.test.tsx
  git commit -m "feat(dosing): add accessible grouped-bar DosingHourlyChart (LAYER2-DATAVIZ-002)"
  ```

### Task 9: Gắn `DosingHourlyChart` vào `DosingHistory.tsx`, thay thế cột đơn trong `DosingTotalCard`

**Files:**
- Modify: `hydragrow-frontend/src/components/dosing/DosingTotalCard.tsx`
- Modify: `hydragrow-frontend/src/components/ui/DosingSummaryCard.test.tsx` (đường dẫn giữ nguyên — kiểm tra lại không có test riêng cho DosingTotalCard trước khi sửa)
- Modify: `hydragrow-frontend/src/pages/DosingHistory.tsx`

- [ ] **Bước 1: Xác nhận `DosingTotalCard` chưa có file test riêng (tránh xoá nhầm)**

  ```bash
  find hydragrow-frontend/src -iname "DosingTotalCard*"
  ```
  Kỳ vọng: chỉ có `DosingTotalCard.tsx`, không có `.test.tsx` — an toàn để sửa phần render mà không cần cập nhật test riêng của nó (hành vi `totalMlToday`/`averageMlPerDay`/`changePercent` đã được test gián tiếp qua `dosingAggregates.test.ts`).

- [ ] **Bước 2: Sửa `DosingTotalCard.tsx` — thay khối cột đơn bằng `DosingHourlyChart`**

  Đổi props: thay `hourlyValues: number[]` bằng `bucketsByPump: Record<PumpField, number[]>` (import type từ `dosingAggregates.ts`), xoá phần tự vẽ `<div className="flex items-end gap-1 h-20 pt-2">...</div>`, thay bằng:
  ```tsx
  import { DosingHourlyChart } from './DosingHourlyChart';
  import type { PumpField } from '../../lib/dosing/dosingAggregates';

  interface DosingTotalCardProps {
      totalMlToday: number;
      bucketsByPump: Record<PumpField, number[]>;
      averageMlPerDay: number;
      changePercent: number;
  }

  export const DosingTotalCard = ({
      totalMlToday,
      bucketsByPump,
      averageMlPerDay,
      changePercent,
  }: DosingTotalCardProps) => {
      return (
          <div className="bg-white border border-line rounded-2xl p-4 space-y-3 shadow-sm">
              <p className="text-[10px] font-bold uppercase tracking-wider text-faint">Tổng lượng châm hôm nay</p>
              <p className="text-3xl font-black text-primary-deep">{totalMlToday.toFixed(0)} ml</p>
              <DosingHourlyChart bucketsByPump={bucketsByPump} />
              <p className="text-xs text-text-muted pt-1">
                  Trung bình 7 ngày: <b>{averageMlPerDay.toFixed(0)} ml/ngày</b>
                  {changePercent !== 0 && (
                      <span className={changePercent > 0 ? 'text-error font-bold' : 'text-status font-bold'}>
                          {' '}({changePercent > 0 ? '+' : ''}{changePercent.toFixed(0)}% so với tuần trước)
                      </span>
                  )}
              </p>
          </div>
      );
  };
  ```
  (`PumpField` cần được `export`-ed từ `dosingAggregates.ts` nếu chưa — kiểm tra: hiện tại `type PumpField` không có từ khoá `export`. Thêm `export` vào định nghĩa `type PumpField` trong `dosingAggregates.ts` như một sửa nhỏ đi kèm bước này.)

- [ ] **Bước 3: Sửa `DosingHistory.tsx` — dùng `hourlyBucketsByPump` thay vì `hourlyBuckets`**

  Đổi import:
  ```typescript
  import { totalMlToday, hourlyBucketsByPump, sevenDayAverage, detectAnomalies, DosingHistoryRangeRecord } from '../lib/dosing/dosingAggregates';
  ```
  Đổi dòng tính toán:
  ```typescript
  const bucketsByPump = hourlyBucketsByPump(records);
  ```
  Đổi prop truyền vào `<DosingTotalCard ... hourlyValues={hourlyValues} ... />` thành:
  ```tsx
  <DosingTotalCard
    totalMlToday={totalToday}
    bucketsByPump={bucketsByPump}
    averageMlPerDay={averageMlPerDay}
    changePercent={changePercentVsPrevious7Days}
  />
  ```

- [ ] **Bước 4: Chạy toàn bộ test liên quan + build type-check**

  ```bash
  npx tsc --noEmit
  npx vitest run src/lib/dosing/ src/components/dosing/ src/pages/DosingHistory.test.tsx 2>/dev/null || npx vitest run src/lib/dosing/ src/components/dosing/
  ```
  (Nếu `DosingHistory.test.tsx` không tồn tại, lệnh fallback chạy đúng phần liên quan — kiểm tra bằng `find hydragrow-frontend/src/pages -iname "DosingHistory.test*"` trước khi quyết định câu lệnh cuối.)

- [ ] **Bước 5: Commit**

  ```bash
  git add hydragrow-frontend/src/components/dosing/DosingTotalCard.tsx \
          hydragrow-frontend/src/pages/DosingHistory.tsx \
          hydragrow-frontend/src/lib/dosing/dosingAggregates.ts
  git commit -m "feat(dosing-history): wire grouped per-pump chart into DosingHistory page (LAYER2-DATAVIZ-003)"
  ```

### Task 10: Sửa `DosingReportCard.tsx` dùng `pumpVisualTheme` (khép guard cho file này)

**Files:**
- Modify: `hydragrow-frontend/src/components/dosing/DosingReportCard.tsx`

- [ ] **Bước 1: Thay khối màu node timeline**

  Thay:
  ```tsx
  className={`w-8 h-8 rounded-full border-4 border-white flex items-center justify-center shadow-md ${
    hasNutrient
      ? 'bg-orange-500 text-white'
      : hasPhUp || hasPhDown
      ? 'bg-fuchsia-600 text-white'
      : 'bg-sky-600 text-white'
  }`}
  ```
  bằng (dùng đúng theme của bơm chiếm ưu thế trong bản ghi, nhất quán với chart legend ở Task 8):
  ```tsx
  import { PUMP_VISUAL_THEME } from '../../lib/dosing/pumpVisualTheme';
  // ...
  className={`w-8 h-8 rounded-full border-4 border-white flex items-center justify-center shadow-md ${
    hasNutrient
      ? PUMP_VISUAL_THEME.nutrient.activeIcon
      : hasPhUp
      ? PUMP_VISUAL_THEME.phUp.activeIcon
      : hasPhDown
      ? PUMP_VISUAL_THEME.phDown.activeIcon
      : PUMP_VISUAL_THEME.aqua.activeIcon
  }`}
  ```
  (Trước đây `hasPhUp || hasPhDown` gộp chung 1 màu `fuchsia` — bản mới tách `hasPhUp` riêng khỏi `hasPhDown` để khớp đúng 2 theme khác nhau `phUp`/`phDown`, khắc phục luôn phần "2 bơm pH cùng 1 màu" ở node timeline mà bản gốc có.)

- [ ] **Bước 2: Thay khối badge ml theo từng bơm**

  Thay 4 khối `<span className="text-orange-700 bg-orange-50 ...">`/`text-purple-700 bg-purple-50`/`text-red-700 bg-red-50`/`text-sky-700 bg-sky-50` bằng:
  ```tsx
  {record.pump_a_ml > 0 && (
    <span className={`px-2 py-0.5 rounded border ${PUMP_VISUAL_THEME.nutrient.badge}`}>
      A: {record.pump_a_ml.toFixed(1)}ml
    </span>
  )}
  {record.pump_b_ml > 0 && (
    <span className={`px-2 py-0.5 rounded border ${PUMP_VISUAL_THEME.nutrient.badge}`}>
      B: {record.pump_b_ml.toFixed(1)}ml
    </span>
  )}
  {record.ph_up_ml > 0 && (
    <span className={`px-2 py-0.5 rounded border ${PUMP_VISUAL_THEME.phUp.badge}`}>
      pH Up: {record.ph_up_ml.toFixed(1)}ml
    </span>
  )}
  {record.ph_down_ml > 0 && (
    <span className={`px-2 py-0.5 rounded border ${PUMP_VISUAL_THEME.phDown.badge}`}>
      pH Down: {record.ph_down_ml.toFixed(1)}ml
    </span>
  )}
  {(dosing.water_in_sec ?? 0) > 0 && (
    <span className={`px-2 py-0.5 rounded border flex items-center gap-1 ${PUMP_VISUAL_THEME.aqua.badge}`}>
      <Waves size={10} /> Cấp nước {dosing.water_in_sec?.toFixed(1)}s
    </span>
  )}
  ```
  (Trước đây pH Down dùng `red-*` — không phải hue trong `pumpVisualTheme`/`OFF_BRAND_HUES` — nay thống nhất về `phDown` = fuchsia, khớp với card điều khiển ở Track A.)

- [ ] **Bước 2: Chạy guard + type-check**

  ```bash
  npx tsc --noEmit
  npx vitest run src/lib/design-lint/hardcodedColors.test.ts
  ```
  Kỳ vọng: `DosingReportCard.tsx` không còn xuất hiện trong danh sách vi phạm.

- [ ] **Bước 3: Commit**

  ```bash
  git add hydragrow-frontend/src/components/dosing/DosingReportCard.tsx
  git commit -m "fix(dosing): DosingReportCard badges use shared pumpVisualTheme, fix ph_down red->fuchsia mismatch (LAYER2-DATAVIZ-004)"
  ```

### Task 11: Sparkline xu hướng cho 4 chỉ số sức khoẻ trong `Analytics.tsx`

**Files:**
- Create: `hydragrow-frontend/src/components/ui/Sparkline.tsx`
- Create: `hydragrow-frontend/src/components/ui/Sparkline.test.tsx`
- Create: `hydragrow-frontend/src/hooks/useHealthHistory.ts`
- Create: `hydragrow-frontend/src/hooks/useHealthHistory.test.ts`
- Modify: `hydragrow-frontend/src/pages/Analytics.tsx`

**Áp dụng data-visualization "Trend Over Time → sparklines (inline)":** không có endpoint lịch sử ở backend cho các chỉ số phần cứng này (chỉ có snapshot hiện tại, poll mỗi 15s — xác nhận qua `refetchInterval: 15000` trong `Analytics.tsx`). Giải pháp đúng phạm vi frontend-only: tích luỹ một ring-buffer trong bộ nhớ phiên làm việc (reset khi tải lại trang — chấp nhận được, đây là chỉ báo xu hướng tức thời, không phải lịch sử dài hạn — lịch sử dài hạn đã có qua Grafana).

- [ ] **Bước 1: Viết test thất bại cho hook ring-buffer**

  ```typescript
  // hydragrow-frontend/src/hooks/useHealthHistory.test.ts
  import { renderHook } from '@testing-library/react';
  import { describe, expect, it } from 'vitest';
  import { pushHealthSample, MAX_HEALTH_SAMPLES } from './useHealthHistory';

  describe('pushHealthSample', () => {
    it('thêm mẫu mới vào cuối mảng', () => {
      const result = pushHealthSample([1, 2, 3], 4);
      expect(result).toEqual([1, 2, 3, 4]);
    });

    it('bỏ giá trị null/undefined, không thêm vào buffer', () => {
      expect(pushHealthSample([1, 2], null)).toEqual([1, 2]);
      expect(pushHealthSample([1, 2], undefined)).toEqual([1, 2]);
    });

    it(`giới hạn tối đa ${MAX_HEALTH_SAMPLES} mẫu — mẫu cũ nhất bị đẩy ra`, () => {
      const full = Array.from({ length: MAX_HEALTH_SAMPLES }, (_, i) => i);
      const result = pushHealthSample(full, 999);
      expect(result).toHaveLength(MAX_HEALTH_SAMPLES);
      expect(result[result.length - 1]).toBe(999);
      expect(result[0]).toBe(1);
    });
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail** — module chưa tồn tại.

- [ ] **Bước 3: Cài đặt hook**

  ```typescript
  // hydragrow-frontend/src/hooks/useHealthHistory.ts
  import { useRef } from 'react';

  /** Số mẫu tối đa giữ trong bộ nhớ phiên — 15s/mẫu * 40 ≈ 10 phút xu hướng gần nhất. */
  export const MAX_HEALTH_SAMPLES = 40;

  export function pushHealthSample(buffer: number[], value: number | null | undefined): number[] {
    if (value === null || value === undefined) return buffer;
    const next = [...buffer, value];
    return next.length > MAX_HEALTH_SAMPLES ? next.slice(next.length - MAX_HEALTH_SAMPLES) : next;
  }

  /**
   * Ring-buffer xu hướng trong bộ nhớ phiên cho 1 chỉ số sức khoẻ phần cứng.
   * Không có endpoint lịch sử ở backend cho các chỉ số này — reset khi tải
   * lại trang là đánh đổi chấp nhận được; lịch sử dài hạn xem qua Grafana
   * (đã có, xem Analytics.tsx phần grafanaUrl).
   */
  export function useHealthHistory() {
    const ref = useRef<Record<string, number[]>>({});

    const record = (key: string, value: number | null | undefined) => {
      ref.current[key] = pushHealthSample(ref.current[key] ?? [], value);
      return ref.current[key];
    };

    return { record };
  }
  ```

- [ ] **Bước 4: Chạy lại, xác nhận pass** — `npx vitest run src/hooks/useHealthHistory.test.ts` → 3/3 PASS.

- [ ] **Bước 5: Viết test thất bại cho `Sparkline`**

  ```tsx
  // hydragrow-frontend/src/components/ui/Sparkline.test.tsx
  import { render } from '@testing-library/react';
  import { describe, expect, it } from 'vitest';
  import { Sparkline } from './Sparkline';

  describe('Sparkline', () => {
    it('render svg với đúng số điểm dữ liệu', () => {
      const { container } = render(<Sparkline values={[1, 3, 2, 5, 4]} label="RSSI WiFi" />);
      const svg = container.querySelector('svg');
      expect(svg).toBeInTheDocument();
      expect(svg).toHaveAttribute('role', 'img');
      expect(svg).toHaveAttribute('aria-label', expect.stringContaining('RSSI WiFi'));
    });

    it('dưới 2 điểm dữ liệu → không render svg (tránh vẽ đường vô nghĩa)', () => {
      const { container } = render(<Sparkline values={[1]} label="RSSI WiFi" />);
      expect(container.querySelector('svg')).not.toBeInTheDocument();
    });
  });
  ```

- [ ] **Bước 6: Chạy test, xác nhận fail.**

- [ ] **Bước 7: Cài đặt `Sparkline`**

  ```tsx
  // hydragrow-frontend/src/components/ui/Sparkline.tsx
  interface SparklineProps {
    values: number[];
    label: string;
    strokeClassName?: string;
  }

  /** Sparkline inline tối giản — data-visualization skill: "Trend Over Time -> sparklines (inline)". */
  export const Sparkline = ({ values, label, strokeClassName = 'stroke-primary' }: SparklineProps) => {
    if (values.length < 2) return null;

    const width = 100;
    const height = 24;
    const min = Math.min(...values);
    const max = Math.max(...values);
    const range = max - min || 1;

    const points = values
      .map((v, i) => {
        const x = (i / (values.length - 1)) * width;
        const y = height - ((v - min) / range) * height;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');

    return (
      <svg
        role="img"
        aria-label={`Xu hướng ${label} trong phiên hiện tại, ${values.length} mẫu gần nhất`}
        viewBox={`0 0 ${width} ${height}`}
        className="w-full h-6"
        preserveAspectRatio="none"
      >
        <polyline
          points={points}
          fill="none"
          className={strokeClassName}
          strokeWidth={1.5}
          strokeLinejoin="round"
          strokeLinecap="round"
        />
      </svg>
    );
  };
  ```

- [ ] **Bước 8: Chạy lại, xác nhận pass** — `npx vitest run src/components/ui/Sparkline.test.tsx` → 2/2 PASS.

- [ ] **Bước 9: Gắn vào `Analytics.tsx`**

  Thêm import:
  ```typescript
  import { Sparkline } from '../components/ui/Sparkline';
  import { useHealthHistory } from '../hooks/useHealthHistory';
  ```
  Ngay sau khai báo `const { data: health, ... } = useQuery<DeviceHealthMetrics>({...})`, thêm:
  ```typescript
  const { record } = useHealthHistory();
  const heapHistory = record('heap', health?.free_heap_bytes ?? null);
  const rssiHistory = record('rssi', health?.wifi_rssi_dbm ?? null);
  const cpuHistory = record('cpu', health?.backend_process_cpu_percent ?? null);
  ```
  Trong mỗi 1 trong 4 thẻ `ui-card` của khối "HARDWARE HEALTH CARDS", thêm `<Sparkline>` ngay dưới dòng giá trị số. Ví dụ cho thẻ RAM (giữ nguyên toàn bộ phần trên, chỉ thêm 1 dòng trước `<p className="text-[11px] text-text-muted">Bộ nhớ heap khả dụng ESP32</p>`):
  ```tsx
  <div className="text-xl font-bold text-primary-deep" data-testid="health-free-heap">
    {isHealthLoading ? '...' : formatHeap(health?.free_heap_bytes)}
  </div>
  <Sparkline values={heapHistory} label="bộ nhớ heap khả dụng" />
  <p className="text-[11px] text-text-muted">Bộ nhớ heap khả dụng ESP32</p>
  ```
  Tương tự cho thẻ WiFi (`rssiHistory`, label "tín hiệu WiFi") và thẻ CPU (`cpuHistory`, label "tải CPU backend"). Thẻ Uptime **không** cần sparkline (uptime luôn tăng tuyến tính khi trạm còn sống — một đường thẳng đi lên không mang thêm thông tin gì so với con số hiện tại; đây là ví dụ đúng của data-visualization skill "Choose the simplest chart that communicates the insight" — nghĩa là đôi khi đúng là *không* vẽ chart).

- [ ] **Bước 10: Chạy lại toàn bộ test của `Analytics.tsx`**

  ```bash
  npx tsc --noEmit
  npx vitest run src/pages/Analytics.test.tsx src/hooks/useHealthHistory.test.ts src/components/ui/Sparkline.test.tsx
  ```

- [ ] **Bước 11: Commit**

  ```bash
  git add hydragrow-frontend/src/components/ui/Sparkline.tsx \
          hydragrow-frontend/src/components/ui/Sparkline.test.tsx \
          hydragrow-frontend/src/hooks/useHealthHistory.ts \
          hydragrow-frontend/src/hooks/useHealthHistory.test.ts \
          hydragrow-frontend/src/pages/Analytics.tsx
  git commit -m "feat(analytics): add in-session health-metric sparklines (LAYER2-DATAVIZ-005)"
  ```

### Task 12: Sửa icon màu hardcode trong `Analytics.tsx`

**Files:**
- Modify: `hydragrow-frontend/src/pages/Analytics.tsx`

- [ ] **Bước 1: Thay 2 dòng vi phạm**

  Dòng 135 (icon RAM) và dòng 315 (icon nhóm "ESP32 Telemetry"): `text-purple-600`/`text-purple-700` → `text-primary` (đã là icon trang trí đơn sắc, không mang ý nghĩa phân loại như bảng màu bơm — dùng token thương hiệu chính là đủ, không cần bảng categorical riêng cho 4 ô sức khoẻ phần cứng vốn đã phân biệt bằng icon + nhãn chữ, không phụ thuộc màu):
  ```tsx
  <Cpu size={16} className="text-primary" />
  ```
  và
  ```tsx
  <Cpu size={16} className="text-primary shrink-0" />
  ```

- [ ] **Bước 2: Chạy guard**

  ```bash
  npx vitest run src/lib/design-lint/hardcodedColors.test.ts
  ```
  Kỳ vọng: `Analytics.tsx` không còn trong danh sách vi phạm.

- [ ] **Bước 3: Commit**

  ```bash
  git add hydragrow-frontend/src/pages/Analytics.tsx
  git commit -m "fix(analytics): decorative icons use --color-primary token instead of hardcoded purple (LAYER2-DATAVIZ-006)"
  ```

---

## Track C — `form-design` (D9 Device Pairing)

**Phạm vi thật đã xác minh:** `DevicePairing.tsx` form nhập tay chỉ kiểm tra `.trim()` khác rỗng trước khi mở overlay xác nhận — không có định dạng gợi ý, không validate on-blur; banner lỗi (`error || formError`) là 1 khối `bg-rose-50` dùng chung cho cả lỗi mạng lẫn lỗi nhập liệu, đặt tách rời khỏi field liên quan (vi phạm "error placement: directly below the field"); `ScanConfirmOverlay.tsx` chỉ có 1 lối thoát khi mã không khớp ("Mã không khớp" → đóng overlay, không gợi ý quét lại ngay).

### Task 13: Validate định dạng Device ID khi rời trường (on blur)

**Files:**
- Create: `hydragrow-frontend/src/lib/pairing/deviceIdValidation.ts`
- Create: `hydragrow-frontend/src/lib/pairing/deviceIdValidation.test.ts`
- Modify: `hydragrow-frontend/src/pages/DevicePairing.tsx`

- [ ] **Bước 1: Viết test thất bại**

  ```typescript
  // hydragrow-frontend/src/lib/pairing/deviceIdValidation.test.ts
  import { describe, expect, it } from 'vitest';
  import { validateDeviceId } from './deviceIdValidation';

  describe('validateDeviceId', () => {
    it('rỗng → lỗi bắt buộc', () => {
      expect(validateDeviceId('')).toBe('Vui lòng nhập Device ID.');
      expect(validateDeviceId('   ')).toBe('Vui lòng nhập Device ID.');
    });

    it('chứa khoảng trắng ở giữa → lỗi định dạng', () => {
      expect(validateDeviceId('hydra station 01')).toBe(
        'Device ID chỉ gồm chữ, số, dấu gạch dưới hoặc gạch ngang, không có khoảng trắng.',
      );
    });

    it('chứa ký tự đặc biệt không hợp lệ → lỗi định dạng', () => {
      expect(validateDeviceId('hydra@station!01')).toBe(
        'Device ID chỉ gồm chữ, số, dấu gạch dưới hoặc gạch ngang, không có khoảng trắng.',
      );
    });

    it('hợp lệ (chữ, số, gạch dưới, gạch ngang) → null', () => {
      expect(validateDeviceId('hydra_station-01')).toBeNull();
      expect(validateDeviceId('HydraStation01')).toBeNull();
    });

    it('URL dạng hydragrow://claim/... vẫn hợp lệ trước khi được parse riêng', () => {
      // Cho phép qua đây; parseDeviceIdFromQr() xử lý trích xuất trước khi validate được gọi trên chuỗi đã trích.
      expect(validateDeviceId('hydra-station-01')).toBeNull();
    });
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail.**

- [ ] **Bước 3: Cài đặt validator**

  ```typescript
  // hydragrow-frontend/src/lib/pairing/deviceIdValidation.ts

  /**
   * Validate định dạng Device ID nhập tay (form-design skill: "Inline
   * validation: validate on blur" + "Error messages: explain what went
   * wrong and how to fix it"). Không validate xem thiết bị có tồn tại hay
   * đã bị người khác claim hay chưa — đó là lỗi phía server, hiển thị
   * riêng qua formError sau khi gọi API (xem Task 14).
   */
  const VALID_DEVICE_ID = /^[a-zA-Z0-9_-]+$/;

  export function validateDeviceId(raw: string): string | null {
    const trimmed = raw.trim();
    if (!trimmed) return 'Vui lòng nhập Device ID.';
    if (!VALID_DEVICE_ID.test(trimmed)) {
      return 'Device ID chỉ gồm chữ, số, dấu gạch dưới hoặc gạch ngang, không có khoảng trắng.';
    }
    return null;
  }
  ```

- [ ] **Bước 4: Chạy lại, xác nhận pass** — 5/5 PASS.

- [ ] **Bước 5: Gắn validate-on-blur vào form nhập tay trong `DevicePairing.tsx`**

  Thêm state và import:
  ```typescript
  import { validateDeviceId } from '../lib/pairing/deviceIdValidation';
  // ...
  const [deviceIdFieldError, setDeviceIdFieldError] = useState<string | null>(null);
  ```

  Sửa khối input Device ID (trong "FORM THÊM THỦ CÔNG"):
  ```tsx
  <div>
    <label htmlFor="manual-device-id" className="block text-xs font-semibold text-primary-deep mb-1">
      Device ID *
    </label>
    <input
      id="manual-device-id"
      type="text"
      placeholder="Ví dụ: hydra_station_01"
      value={newDeviceId}
      onChange={(e) => setNewDeviceId(e.target.value)}
      onBlur={() => setDeviceIdFieldError(newDeviceId ? validateDeviceId(newDeviceId) : null)}
      aria-invalid={Boolean(deviceIdFieldError)}
      aria-describedby={deviceIdFieldError ? 'manual-device-id-error' : undefined}
      className={`w-full px-3 py-2 text-sm rounded-xl border bg-white focus:outline-none font-mono text-xs ${
        deviceIdFieldError ? 'border-error focus:border-error' : 'border-line focus:border-primary'
      }`}
    />
    {deviceIdFieldError && (
      <p id="manual-device-id-error" className="mt-1 text-[11px] text-error font-medium">
        {deviceIdFieldError}
      </p>
    )}
  </div>
  ```
  (Placeholder được giữ nguyên là ví dụ hợp lệ — không cần đổi vì đã đúng theo form-design skill: mô tả rõ định dạng mong đợi.)

  Sửa nút "Tiếp tục xác nhận ghép nối" — chặn tiếp tục nếu còn lỗi định dạng, không chỉ kiểm tra rỗng:
  ```tsx
  <button
    onClick={() => {
      const err = validateDeviceId(newDeviceId);
      setDeviceIdFieldError(err);
      if (!err) {
        setPendingDeviceId(newDeviceId.trim());
        setShowConfirmOverlay(true);
      }
    }}
    disabled={submitting || !newDeviceId.trim()}
    className="ui-btn-primary flex items-center gap-2"
  >
    <Sparkles size={16} /> Tiếp tục xác nhận ghép nối
  </button>
  ```

- [ ] **Bước 6: Chạy test hiện có của trang, xác nhận không vỡ**

  ```bash
  npx vitest run src/pages/DevicePairing.test.tsx
  npx tsc --noEmit
  ```

- [ ] **Bước 7: Commit**

  ```bash
  git add hydragrow-frontend/src/lib/pairing/deviceIdValidation.ts \
          hydragrow-frontend/src/lib/pairing/deviceIdValidation.test.ts \
          hydragrow-frontend/src/pages/DevicePairing.tsx
  git commit -m "feat(pairing): inline on-blur Device ID format validation (LAYER2-FORMUX-001)"
  ```

### Task 14: Chuyển banner lỗi trang sang `Banner` dùng chung, tách lỗi theo ngữ cảnh

**Files:**
- Modify: `hydragrow-frontend/src/pages/DevicePairing.tsx`

**Áp dụng form-design "Server-side errors: surface inline to the field if possible; summarize at the top if multiple fields are affected":** `error` (từ `useOwnedDevices`, lỗi tải danh sách — không thuộc field nào) hiển thị dạng banner đầu trang; `formError` (từ `executeClaim`/`unclaimDevice`/`saveRename` — luôn xảy ra sau khi bấm nút trong ngữ cảnh cụ thể) chuyển vào đúng vị trí gây ra nó thay vì gộp chung 1 khối ở đầu trang.

- [ ] **Bước 1: Thay khối banner gộp hiện tại**

  Xoá:
  ```tsx
  {(error || formError) && (
    <div className="p-3.5 bg-rose-50 border border-rose-200 text-rose-700 rounded-xl text-xs font-medium">
      {error || formError}
    </div>
  )}
  ```
  Thay bằng (chỉ còn lỗi tải danh sách — lỗi không gắn với 1 hành động cụ thể nào, đúng nghĩa "page-level" theo error-handling-ux skill):
  ```tsx
  import { Banner } from '../components/ui/Banner';
  // ...
  {error && (
    <Banner tone="danger" title="Không thể tải danh sách thiết bị">
      {error}
    </Banner>
  )}
  ```

- [ ] **Bước 2: Đưa `formError` vào đúng ngữ cảnh — bên trong `ScanConfirmOverlay`**

  `formError` hiện chỉ được set bởi `executeClaim` (lỗi ghép nối — xảy ra trong khi overlay đang mở), `unclaimDevice`, `saveRename` (lỗi trên các thẻ thiết bị đã liên kết). Thay vì 1 biến `formError` dùng chung cho cả 3 hành động khác nhau, tách theo đúng nơi hiển thị:

  Trong `executeClaim`, thay `setFormError(e.message || 'Lỗi ghép nối thiết bị')` — **giữ nguyên set formError** (vẫn cần để hiển thị ngay trong overlay), nhưng truyền xuống `ScanConfirmOverlay` thay vì hiển thị ở đầu trang. Sửa lời gọi component:
  ```tsx
  {showConfirmOverlay && pendingDeviceId && (
    <ScanConfirmOverlay
      deviceId={pendingDeviceId}
      initialLabel={newLabel}
      isSubmitting={submitting}
      errorMessage={formError}
      onConfirm={(devId, label) => executeClaim(devId, label || null)}
      onCancel={() => {
        setShowConfirmOverlay(false);
        setPendingDeviceId(null);
        setFormError(null);
      }}
    />
  )}
  ```
  Trong `unclaimDevice`/`saveRename`, thay `setFormError(e.message)` bằng `toast.error(e.message)` **thôi** (toast đã tồn tại sẵn ngay dòng dưới ở cả 2 hàm — hiện đang gọi cả `setFormError` lẫn `toast.error` cho cùng 1 lỗi, hiển thị trùng lặp 2 lần; xoá dòng `setFormError(e.message)` trong 2 hàm này, giữ nguyên `toast.error(e.message)` — action tức thời như xoá/đổi tên thiết bị phù hợp với toast hơn là banner tồn tại lâu, đúng "Recovery: offer retry... Undo for accidental actions" mà không cần chiếm chỗ cố định trên trang).

- [ ] **Bước 3: Thêm prop `errorMessage` vào `ScanConfirmOverlay`**

  Trong `ScanConfirmOverlay.tsx`, thêm vào interface:
  ```typescript
  export interface ScanConfirmOverlayProps {
    deviceId: string;
    initialLabel?: string;
    errorMessage?: string | null;
    onConfirm: (deviceId: string, label: string) => void | Promise<void>;
    onCancel: () => void;
    isSubmitting?: boolean;
  }
  ```
  Thêm tham số vào destructure và hiển thị ngay trên hàng nút hành động (đúng "near the source" — bên trong modal nơi hành động Xác nhận đang diễn ra, không phải ở trang phía sau):
  ```tsx
  export const ScanConfirmOverlay: React.FC<ScanConfirmOverlayProps> = ({
    deviceId,
    initialLabel = '',
    errorMessage = null,
    onConfirm,
    onCancel,
    isSubmitting = false,
  }) => {
    // ... (giữ nguyên phần trên)

    return (
      <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in" data-testid="scan-confirm-overlay">
        <div className="ui-card max-w-md w-full p-6 space-y-6 shadow-2xl border border-line animate-in zoom-in-95">
          {/* ... phần Device ID / confirm code / label input giữ nguyên ... */}

          {errorMessage && (
            <Banner tone="danger" title="Không thể ghép nối">
              {errorMessage}
            </Banner>
          )}

          {/* ... phần action buttons giữ nguyên ... */}
        </div>
      </div>
    );
  };
  ```
  Thêm import `Banner` ở đầu file: `import { Banner } from '../ui/Banner';`.

- [ ] **Bước 4: Chạy test hiện có, xác nhận không vỡ**

  ```bash
  npx tsc --noEmit
  npx vitest run src/pages/DevicePairing.test.tsx
  ```

- [ ] **Bước 5: Commit**

  ```bash
  git add hydragrow-frontend/src/pages/DevicePairing.tsx \
          hydragrow-frontend/src/components/pairing/ScanConfirmOverlay.tsx
  git commit -m "fix(pairing): move claim error next to the action that caused it, dedupe toast+banner (LAYER2-FORMUX-002)"
  ```

### Task 15: Sửa `ScanConfirmOverlay.tsx` — hex/hue hardcode → token, thêm lối "Quét lại"

**Files:**
- Modify: `hydragrow-frontend/src/components/pairing/ScanConfirmOverlay.tsx`
- Modify: `hydragrow-frontend/src/pages/DevicePairing.tsx`

**Áp dụng error-handling-ux "Recovery: Offer retry for transient failures; Provide alternative paths":** hiện tại nút "Mã không khớp" chỉ đóng overlay — người dùng phải tự nhớ bấm lại "Quét mã QR trạm" từ đầu. Thêm callback `onRetryScan` tuỳ chọn để khi ở luồng quét QR (không phải nhập tay), đóng overlay **và** mở lại camera ngay.

- [ ] **Bước 1: Sửa màu hardcode trong `ScanConfirmOverlay.tsx`**

  Thay:
  ```tsx
  className="ui-btn-md border border-line text-rose-600 bg-white hover:bg-rose-50 flex items-center justify-center gap-1.5"
  ```
  bằng:
  ```tsx
  className="ui-btn-md border border-line text-error bg-white hover:bg-danger-bg flex items-center justify-center gap-1.5"
  ```

- [ ] **Bước 2: Thêm prop `onRetryScan` tuỳ chọn**

  ```typescript
  export interface ScanConfirmOverlayProps {
    deviceId: string;
    initialLabel?: string;
    errorMessage?: string | null;
    onConfirm: (deviceId: string, label: string) => void | Promise<void>;
    onCancel: () => void;
    /** Khi có: nút "Mã không khớp" đổi thành "Quét lại" và gọi hàm này thay vì chỉ đóng overlay. */
    onRetryScan?: () => void;
    isSubmitting?: boolean;
  }
  ```
  Sửa nút huỷ:
  ```tsx
  <button
    type="button"
    onClick={onRetryScan ?? onCancel}
    disabled={isSubmitting}
    className="ui-btn-md border border-line text-error bg-white hover:bg-danger-bg flex items-center justify-center gap-1.5"
  >
    <X size={16} /> {onRetryScan ? 'Quét lại' : 'Mã không khớp'}
  </button>
  ```

- [ ] **Bước 3: Nối `onRetryScan` từ `DevicePairing.tsx` — chỉ khi mã vừa nhận đến từ camera, không phải nhập tay**

  Cần phân biệt nguồn gốc `pendingDeviceId` (từ quét QR hay nhập tay). Thêm state:
  ```typescript
  const [pendingDeviceIdSource, setPendingDeviceIdSource] = useState<'qr' | 'manual'>('manual');
  ```
  Trong callback quét QR thành công (`startScanner`), ngay chỗ `setPendingDeviceId(extracted); setShowConfirmOverlay(true);`, thêm:
  ```typescript
  setPendingDeviceIdSource('qr');
  ```
  Trong nút "Tiếp tục xác nhận ghép nối" (nhập tay, đã sửa ở Task 13), thêm:
  ```typescript
  setPendingDeviceIdSource('manual');
  ```
  Truyền xuống overlay:
  ```tsx
  <ScanConfirmOverlay
    deviceId={pendingDeviceId}
    initialLabel={newLabel}
    isSubmitting={submitting}
    errorMessage={formError}
    onConfirm={(devId, label) => executeClaim(devId, label || null)}
    onCancel={() => {
      setShowConfirmOverlay(false);
      setPendingDeviceId(null);
      setFormError(null);
    }}
    onRetryScan={
      pendingDeviceIdSource === 'qr'
        ? () => {
            setShowConfirmOverlay(false);
            setPendingDeviceId(null);
            setFormError(null);
            startScanner();
          }
        : undefined
    }
  />
  ```

- [ ] **Bước 4: Chạy test, xác nhận không vỡ + guard sạch cho file này**

  ```bash
  npx tsc --noEmit
  npx vitest run src/pages/DevicePairing.test.tsx
  npx vitest run src/lib/design-lint/hardcodedColors.test.ts
  ```
  Kỳ vọng: `ScanConfirmOverlay.tsx` không còn trong danh sách vi phạm guard.

- [ ] **Bước 5: Commit**

  ```bash
  git add hydragrow-frontend/src/components/pairing/ScanConfirmOverlay.tsx \
          hydragrow-frontend/src/pages/DevicePairing.tsx
  git commit -m "fix(pairing): token colors + one-tap rescan recovery on code mismatch (LAYER2-FORMUX-003)"
  ```

### Task 16: Khép guard cho phần còn lại của `DevicePairing.tsx`

**Files:**
- Modify: `hydragrow-frontend/src/pages/DevicePairing.tsx`

- [ ] **Bước 1: Sửa nút "Dừng quét camera"**

  Thay:
  ```tsx
  className="ui-btn-md border border-rose-300 text-rose-600 bg-rose-50 hover:bg-rose-100 flex items-center gap-2"
  ```
  bằng:
  ```tsx
  className="ui-btn-md border border-error/40 text-error bg-danger-bg hover:bg-danger-bg/70 flex items-center gap-2"
  ```

- [ ] **Bước 2: Sửa nút xoá liên kết thiết bị (hover state)**

  Thay:
  ```tsx
  className="p-2 text-text-muted hover:text-rose-600 hover:bg-rose-50 rounded-xl transition-colors"
  ```
  bằng:
  ```tsx
  className="p-2 text-text-muted hover:text-error hover:bg-danger-bg rounded-xl transition-colors"
  ```

- [ ] **Bước 3: Chạy guard đầy đủ cho file**

  ```bash
  npx vitest run src/lib/design-lint/hardcodedColors.test.ts
  npx tsc --noEmit
  ```
  Kỳ vọng: `DevicePairing.tsx` không còn xuất hiện trong danh sách vi phạm — cùng với Task 15, Track C khép hoàn toàn guard cho phạm vi D9.

- [ ] **Bước 4: Commit**

  ```bash
  git add hydragrow-frontend/src/pages/DevicePairing.tsx
  git commit -m "fix(pairing): remaining rose-* hardcodes -> --color-error/--color-danger-bg tokens (LAYER2-FORMUX-004)"
  ```

---

## Track D — `search-ux` (Journal / System Log)

**Phạm vi thật đã xác minh:** `Journal.tsx` chỉ là `TabShell` bọc `SystemLog` (tab "Sự kiện") + `Analytics` (tab "Phân tích") — nội dung tìm kiếm thật nằm trong `SystemLog.tsx` + `HealthSummaryBar.tsx`. `filterEventsBySearch()` (trong `lib/logs/eventGrouping.ts`) chỉ so khớp substring không phân biệt hoa/thường trên `title/message/category/reason` — không có nút xoá tìm kiếm, không đếm kết quả, không tô đậm từ khớp, trạng thái rỗng chung cho cả "không có sự kiện" lẫn "tìm không ra" mà không lặp lại từ khoá đã gõ.

### Task 17: Thêm nút xoá tìm kiếm + đếm kết quả trong `HealthSummaryBar`

**Files:**
- Modify: `hydragrow-frontend/src/components/logs/HealthSummaryBar.tsx`
- Create: `hydragrow-frontend/src/components/logs/HealthSummaryBar.test.tsx`

- [ ] **Bước 1: Viết test thất bại (chưa có file test cho component này — tạo mới)**

  ```tsx
  // hydragrow-frontend/src/components/logs/HealthSummaryBar.test.tsx
  import { render, screen, fireEvent } from '@testing-library/react';
  import { describe, expect, it, vi } from 'vitest';
  import { HealthSummaryBar } from './HealthSummaryBar';

  describe('HealthSummaryBar', () => {
    it('không hiện nút xoá khi search rỗng', () => {
      render(<HealthSummaryBar mode="important" onModeChange={vi.fn()} search="" onSearchChange={vi.fn()} />);
      expect(screen.queryByLabelText('Xoá tìm kiếm')).not.toBeInTheDocument();
    });

    it('hiện nút xoá + số kết quả khi có search và resultCount', () => {
      render(
        <HealthSummaryBar
          mode="important"
          onModeChange={vi.fn()}
          search="bơm"
          onSearchChange={vi.fn()}
          resultCount={7}
        />,
      );
      expect(screen.getByLabelText('Xoá tìm kiếm')).toBeInTheDocument();
      expect(screen.getByText('7 kết quả')).toBeInTheDocument();
    });

    it('bấm nút xoá gọi onSearchChange("")', () => {
      const onSearchChange = vi.fn();
      render(
        <HealthSummaryBar mode="important" onModeChange={vi.fn()} search="bơm" onSearchChange={onSearchChange} resultCount={0} />,
      );
      fireEvent.click(screen.getByLabelText('Xoá tìm kiếm'));
      expect(onSearchChange).toHaveBeenCalledWith('');
    });
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail** (prop `resultCount` chưa tồn tại, nút xoá chưa có).

- [ ] **Bước 3: Sửa component**

  Thêm prop `resultCount?: number` vào interface, thêm nút xoá + đếm kết quả (search-ux skill: "Show a clear/reset button once a query is entered"; "Show result count"):
  ```tsx
  import { Search, ShieldAlert, FlaskConical, Waves, AlertTriangle, X } from 'lucide-react';

  interface HealthSummaryBarProps {
    summary?: SystemHealthSummary;
    mode: LogViewMode;
    onModeChange: (mode: LogViewMode) => void;
    search: string;
    onSearchChange: (value: string) => void;
    resultCount?: number;
  }

  export const HealthSummaryBar = ({ summary, mode, onModeChange, search, onSearchChange, resultCount }: HealthSummaryBarProps) => {
    return (
      <div className="ui-card space-y-4">
        {/* ... 4 ô grid summary giữ nguyên, sửa màu ở Task 19 ... */}

        <div className="flex flex-col md:flex-row items-stretch md:items-center gap-3">
          <div className="relative flex-1 min-w-0">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-faint" />
            <input
              type="search"
              value={search}
              onChange={(e) => onSearchChange(e.target.value)}
              placeholder="Tìm theo tiêu đề, nội dung, danh mục..."
              className="ui-input pl-8 pr-8"
              aria-label="Tìm kiếm nhật ký"
            />
            {search && (
              <button
                type="button"
                onClick={() => onSearchChange('')}
                aria-label="Xoá tìm kiếm"
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-faint hover:text-primary-deep"
              >
                <X size={14} />
              </button>
            )}
          </div>
          {search && resultCount !== undefined && (
            <span className="text-[11px] font-semibold text-text-muted shrink-0">
              {resultCount} kết quả
            </span>
          )}
          <div className="flex items-center gap-2 shrink-0">
            <ShieldAlert size={14} className="text-primary" />
            <Switch
              size="sm"
              checked={mode === 'all_technical'}
              onChange={(checked) => onModeChange(checked ? 'all_technical' : 'important')}
              label={mode === 'all_technical' ? 'Toàn bộ kỹ thuật' : 'Quan trọng'}
            />
          </div>
        </div>
      </div>
    );
  };
  ```

- [ ] **Bước 4: Truyền `resultCount` từ `SystemLog.tsx`**

  Trong `SystemLog.tsx`, sửa lời gọi:
  ```tsx
  <HealthSummaryBar
    summary={healthSummary}
    mode={mode}
    onModeChange={setMode}
    search={search}
    onSearchChange={setSearch}
    resultCount={search ? visibleRows.length : undefined}
  />
  ```
  (Chỉ truyền `resultCount` khi đang có từ khoá tìm kiếm — không hiển thị "N kết quả" khi người dùng chỉ đang lọc theo filter mà chưa gõ gì, tránh nhiễu thông tin không liên quan tới hành động tìm kiếm.)

- [ ] **Bước 5: Chạy lại toàn bộ, xác nhận pass**

  ```bash
  npx vitest run src/components/logs/HealthSummaryBar.test.tsx src/pages/SystemLog.test.tsx
  npx tsc --noEmit
  ```

- [ ] **Bước 6: Commit**

  ```bash
  git add hydragrow-frontend/src/components/logs/HealthSummaryBar.tsx \
          hydragrow-frontend/src/components/logs/HealthSummaryBar.test.tsx \
          hydragrow-frontend/src/pages/SystemLog.tsx
  git commit -m "feat(journal): add clear-search button and live result count (LAYER2-SEARCHUX-001)"
  ```

### Task 18: Trạng thái rỗng nhắc lại đúng từ khoá đã tìm + lối thoát nhanh

**Files:**
- Modify: `hydragrow-frontend/src/pages/SystemLog.tsx`

**Áp dụng search-ux "Zero-Results State — Confirm what was searched; Never show a blank page":**

- [ ] **Bước 1: Sửa khối `StateView` rỗng**

  Thay:
  ```tsx
  ) : visibleRows.length === 0 ? (
    <StateView
      icon={Zap}
      title="Dòng thời gian trống"
      description="Chưa ghi nhận khoảnh khắc nào khớp bộ lọc/tìm kiếm hiện tại."
    />
  ) : (
  ```
  bằng:
  ```tsx
  ) : visibleRows.length === 0 ? (
    <StateView
      icon={Zap}
      title={search ? `Không tìm thấy kết quả cho "${search}"` : 'Dòng thời gian trống'}
      description={
        search
          ? 'Thử từ khoá ngắn hơn, kiểm tra chính tả, hoặc đổi bộ lọc danh mục đang chọn.'
          : 'Chưa ghi nhận khoảnh khắc nào khớp bộ lọc hiện tại.'
      }
      action={
        search ? (
          <button
            type="button"
            onClick={() => setSearch('')}
            className="text-xs font-semibold text-primary hover:text-primary-deep"
          >
            Xoá tìm kiếm
          </button>
        ) : filter !== 'all' ? (
          <button
            type="button"
            onClick={() => setFilter('all')}
            className="text-xs font-semibold text-primary hover:text-primary-deep"
          >
            Xem tất cả danh mục
          </button>
        ) : undefined
      }
    />
  ) : (
  ```

- [ ] **Bước 2: Chạy test hiện có**

  ```bash
  npx vitest run src/pages/SystemLog.test.tsx
  npx tsc --noEmit
  ```

- [ ] **Bước 3: Commit**

  ```bash
  git add hydragrow-frontend/src/pages/SystemLog.tsx
  git commit -m "feat(journal): zero-results state echoes the query and offers a way out (LAYER2-SEARCHUX-002)"
  ```

### Task 19: Tô đậm từ khớp trong kết quả + khép guard màu cho các card sự kiện

**Files:**
- Create: `hydragrow-frontend/src/lib/logs/highlightMatch.ts`
- Create: `hydragrow-frontend/src/lib/logs/highlightMatch.test.ts`
- Create: `hydragrow-frontend/src/lib/logs/eventCategoryTheme.ts`
- Modify: `hydragrow-frontend/src/components/logs/EventLogCard.tsx`
- Modify: `hydragrow-frontend/src/components/logs/CycleEventCard.tsx`
- Modify: `hydragrow-frontend/src/components/logs/MetadataRenderers.tsx`
- Modify: `hydragrow-frontend/src/components/ui/FsmStatusBadge.tsx`
- Modify: `hydragrow-frontend/src/components/logs/HealthSummaryBar.tsx`
- Modify: `hydragrow-frontend/src/pages/SystemLog.tsx`

**Áp dụng search-ux "Highlight the query term within suggestions/results":** hiện `EventLogCard` không nhận `search` nên không thể tô đậm. Cùng lúc, đây là các file còn lại vi phạm `hardcodedColors` trong phạm vi Journal — gộp vào 1 task vì cùng thuộc 1 nhóm màu "phân loại theo category sự kiện" (EC dosing/pH dosing/nước/cảnh báo) giống hệt vấn đề "mỗi nơi tự vẽ 1 bảng màu" đã gặp ở Track A, nên xử lý bằng đúng giải pháp đã dùng ở đó: một module `.ts` dùng chung.

- [ ] **Bước 1: Viết test thất bại cho `highlightMatch`**

  ```typescript
  // hydragrow-frontend/src/lib/logs/highlightMatch.test.ts
  import { describe, expect, it } from 'vitest';
  import { splitByMatch } from './highlightMatch';

  describe('splitByMatch', () => {
    it('không có query → trả về 1 đoạn, matched=false', () => {
      expect(splitByMatch('Châm dinh dưỡng A', '')).toEqual([
        { text: 'Châm dinh dưỡng A', matched: false },
      ]);
    });

    it('query khớp giữa chuỗi → tách 3 đoạn, không phân biệt hoa/thường', () => {
      expect(splitByMatch('Châm dinh dưỡng A', 'DINH')).toEqual([
        { text: 'Châm ', matched: false },
        { text: 'dinh', matched: true },
        { text: ' dưỡng A', matched: false },
      ]);
    });

    it('query không khớp → trả về nguyên chuỗi, matched=false', () => {
      expect(splitByMatch('Châm dinh dưỡng A', 'xyz')).toEqual([
        { text: 'Châm dinh dưỡng A', matched: false },
      ]);
    });
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail.**

- [ ] **Bước 3: Cài đặt `highlightMatch.ts`**

  ```typescript
  // hydragrow-frontend/src/lib/logs/highlightMatch.ts

  export interface MatchSegment {
    text: string;
    matched: boolean;
  }

  /** Tách 1 chuỗi thành các đoạn khớp/không khớp với query, không phân biệt hoa/thường — dùng để tô đậm kết quả tìm kiếm (search-ux skill). */
  export function splitByMatch(text: string, query: string): MatchSegment[] {
    const q = query.trim();
    if (!q) return [{ text, matched: false }];

    const idx = text.toLowerCase().indexOf(q.toLowerCase());
    if (idx === -1) return [{ text, matched: false }];

    const segments: MatchSegment[] = [];
    if (idx > 0) segments.push({ text: text.slice(0, idx), matched: false });
    segments.push({ text: text.slice(idx, idx + q.length), matched: true });
    if (idx + q.length < text.length) segments.push({ text: text.slice(idx + q.length), matched: false });
    return segments;
  }
  ```

- [ ] **Bước 4: Chạy lại, xác nhận pass** — 3/3 PASS.

- [ ] **Bước 5: Tạo `eventCategoryTheme.ts` (module `.ts` dùng chung, cùng lý do kiến trúc như `pumpVisualTheme.ts` ở Track A — xem PATTERN-LIBRARY.md mục 5 sẽ dẫn chiếu sang đây)**

  ```typescript
  // hydragrow-frontend/src/lib/logs/eventCategoryTheme.ts

  /**
   * Nguồn sự thật duy nhất cho màu phân loại sự kiện nhật ký (EC dosing / pH
   * dosing / nước / cảnh báo). Trước khi có module này, HealthSummaryBar,
   * EventLogCard, CycleEventCard, MetadataRenderers, FsmStatusBadge mỗi nơi
   * tự chọn 1 hue Tailwind rời rạc cho cùng khái niệm — file .ts thuần, nằm
   * ngoài phạm vi quét của hardcodedColors.test.ts (chỉ liệt kê *.tsx), lý
   * do kiến trúc giống hệt src/lib/dosing/pumpVisualTheme.ts (Track A).
   */
  export type EventCategoryThemeKey = 'ecDosing' | 'phDosing' | 'water' | 'warning' | 'automation' | 'device';

  export const EVENT_CATEGORY_THEME: Record<EventCategoryThemeKey, { icon: string; badge: string }> = {
    ecDosing: { icon: 'text-fuchsia-700', badge: 'text-fuchsia-700 bg-fuchsia-50 border-fuchsia-200' },
    phDosing: { icon: 'text-violet-700', badge: 'text-violet-700 bg-violet-50 border-violet-200' },
    water: { icon: 'text-sky-700', badge: 'text-sky-700 bg-sky-50 border-sky-200' },
    warning: { icon: 'text-warn-deep', badge: 'text-warn-deep bg-warning-bg border-warn-deep/25' },
    automation: { icon: 'text-primary', badge: 'text-primary bg-pill border-primary/25' },
    device: { icon: 'text-cyan-700', badge: 'text-cyan-700 bg-cyan-50 border-cyan-200' },
  };
  ```

- [ ] **Bước 6: Sửa `HealthSummaryBar.tsx` — dùng theme thay vì hue rời rạc**

  Thay 4 icon hardcode (`text-cyan-700`, `text-purple-700`, `text-sky-700`, `text-amber-700`) bằng:
  ```tsx
  import { EVENT_CATEGORY_THEME } from '../../lib/logs/eventCategoryTheme';
  // ...
  <FlaskConical size={14} className={`${EVENT_CATEGORY_THEME.ecDosing.icon} shrink-0`} />
  {/* ... */}
  <FlaskConical size={14} className={`${EVENT_CATEGORY_THEME.phDosing.icon} shrink-0`} />
  {/* ... */}
  <Waves size={14} className={`${EVENT_CATEGORY_THEME.water.icon} shrink-0`} />
  {/* ... */}
  <AlertTriangle size={14} className={`${EVENT_CATEGORY_THEME.warning.icon} shrink-0`} />
  ```

- [ ] **Bước 7: Sửa `EventLogCard.tsx` — dùng theme + thêm highlight**

  Xem cấu trúc hiện có trước khi sửa (component nhận `ev: SystemEvent`, chưa nhận `search`):
  ```bash
  sed -n '1,90p' hydragrow-frontend/src/components/logs/EventLogCard.tsx
  ```
  Thêm prop `search?: string` vào interface của `EventLogCard`, dùng `splitByMatch` để render tiêu đề:
  ```tsx
  import { splitByMatch } from '../../lib/logs/highlightMatch';
  import { EVENT_CATEGORY_THEME } from '../../lib/logs/eventCategoryTheme';

  interface EventLogCardProps {
    ev: SystemEvent;
    idx: number;
    search?: string;
    onOpenDetail: (ev: SystemEvent) => void;
    onAcknowledge: (ev: SystemEvent) => void;
  }

  const HighlightedText = ({ text, query }: { text: string; query?: string }) => (
    <>
      {splitByMatch(text, query ?? '').map((seg, i) =>
        seg.matched ? (
          <mark key={i} className="bg-warning-bg text-warn-deep rounded-sm px-0.5">
            {seg.text}
          </mark>
        ) : (
          <span key={i}>{seg.text}</span>
        ),
      )}
    </>
  );
  ```
  Áp dụng `<HighlightedText text={ev.title} query={search} />` tại vị trí tiêu đề sự kiện đang render trực tiếp `{ev.title}` (giữ nguyên toàn bộ phần layout xung quanh, chỉ bọc phần text). Thay 2 dòng màu badge hardcode (`text-cyan-700`/`bg-cyan-400`/`border-cyan-500`, `text-purple-700`/`bg-purple-400`/`border-purple-500`, và cặp `bg-cyan-50`/`border-cyan-200`, `bg-purple-50`/`border-purple-200` ở khối badge khác) bằng `EVENT_CATEGORY_THEME.device.badge`/`EVENT_CATEGORY_THEME.phDosing.badge` tương ứng đúng ngữ nghĩa từng badge đang biểu diễn (đọc kỹ ngữ cảnh từng dòng trong file thật trước khi thay, vì 2 khối màu ở dòng 37 và dòng 62-63 biểu diễn 2 khái niệm khác nhau trong cùng file — không thay máy móc theo thứ tự xuất hiện).

- [ ] **Bước 8: Truyền `search` xuống `EventLogCard` từ `SystemLog.tsx`**

  Tại lời gọi `<EventLogCard key={row.event.id} ev={row.event} idx={globalIdx} onOpenDetail={setSelectedEvent} onAcknowledge={handleAcknowledge} />`, thêm `search={search}`.

- [ ] **Bước 9: Sửa `CycleEventCard.tsx`, `MetadataRenderers.tsx`, `FsmStatusBadge.tsx` — thay hue hardcode bằng `EVENT_CATEGORY_THEME`**

  Cùng nguyên tắc Bước 7: đọc đúng ngữ cảnh từng dòng đã liệt kê trong baseline (Task 0 Bước 1) trong 3 file này, map sang đúng key ngữ nghĩa (`device`/`phDosing`/`automation`/`warning`) — không đổi giao diện, chỉ đổi nguồn màu về token/theme dùng chung.

- [ ] **Bước 10: Chạy toàn bộ test liên quan + guard**

  ```bash
  npx tsc --noEmit
  npx vitest run src/lib/logs/ src/components/logs/ src/pages/SystemLog.test.tsx
  npx vitest run src/lib/design-lint/hardcodedColors.test.ts
  ```
  Kỳ vọng: guard chỉ còn 3 file tracked-debt từ Task 0 (`SensorBentoCard.tsx`, `ConfigBackup.tsx`, `RecipeBuilder.tsx`) — toàn bộ nhóm Journal/SystemLog đã sạch.

- [ ] **Bước 11: Commit**

  ```bash
  git add hydragrow-frontend/src/lib/logs/highlightMatch.ts \
          hydragrow-frontend/src/lib/logs/highlightMatch.test.ts \
          hydragrow-frontend/src/lib/logs/eventCategoryTheme.ts \
          hydragrow-frontend/src/components/logs/EventLogCard.tsx \
          hydragrow-frontend/src/components/logs/CycleEventCard.tsx \
          hydragrow-frontend/src/components/logs/MetadataRenderers.tsx \
          hydragrow-frontend/src/components/ui/FsmStatusBadge.tsx \
          hydragrow-frontend/src/components/logs/HealthSummaryBar.tsx \
          hydragrow-frontend/src/pages/SystemLog.tsx
  git commit -m "feat(journal): highlight matched search term, unify event-category palette (LAYER2-SEARCHUX-003)"
  ```

---

## Track E — `zeigarnik-effect` + `peak-end-rule` (Crop Seasons)

**Phạm vi thật đã xác minh:** `ActiveSeasonCard.tsx` đã có progress bar theo giai đoạn (elapsed/totalDays) nhưng không có checklist các giai đoạn (chỉ hiện tên giai đoạn hiện tại, không thấy được "còn bao nhiêu bước nữa"). Kết thúc mùa vụ (`handleEnd` → `window.confirm()` gốc → `onEndSeason()`) chuyển thẳng về `SeasonHistoryList` như 1 dòng bình thường — không có màn tổng kết; `CreateSeasonForm` (màn nhập liệu trống) hiện lại ngay lập tức, đúng kiểu "ending on an administrative screen" mà peak-end-rule skill gọi là phản-pattern.

### Task 20: Hàm thuần cho checklist giai đoạn + số ngày còn lại (Zeigarnik)

**Files:**
- Modify: `hydragrow-frontend/src/lib/seasons/seasonProgress.ts`
- Modify: `hydragrow-frontend/src/lib/seasons/seasonProgress.test.ts`

- [ ] **Bước 1: Xem test hiện có trước khi thêm (không sửa test cũ)**

  ```bash
  cat hydragrow-frontend/src/lib/seasons/seasonProgress.test.ts
  ```

- [ ] **Bước 2: Thêm test thất bại**

  ```typescript
  import { stageChecklist, remainingDays } from './seasonProgress';
  import type { CropStage } from '../../types/models';

  const stages: CropStage[] = [
    { name: 'Nảy mầm', duration_sec: 3 * 86400, ec_target: 0.8, ph_target: 6.0 } as CropStage,
    { name: 'Sinh trưởng', duration_sec: 10 * 86400, ec_target: 1.6, ph_target: 5.8 } as CropStage,
    { name: 'Ra hoa', duration_sec: 7 * 86400, ec_target: 2.0, ph_target: 6.2 } as CropStage,
  ];

  describe('remainingDays', () => {
    it('còn lại = tổng - đã trôi qua, không âm', () => {
      expect(remainingDays(20, 12)).toBe(8);
      expect(remainingDays(20, 25)).toBe(0);
    });
  });

  describe('stageChecklist', () => {
    it('đánh dấu done/current/upcoming đúng theo currentStageIndex', () => {
      const result = stageChecklist(stages, 1, 5);
      expect(result).toEqual([
        { name: 'Nảy mầm', status: 'done' },
        { name: 'Sinh trưởng', status: 'current' },
        { name: 'Ra hoa', status: 'upcoming' },
      ]);
    });

    it('giai đoạn đầu tiên → chỉ current + upcoming, không có done', () => {
      const result = stageChecklist(stages, 0, 1);
      expect(result.map((s) => s.status)).toEqual(['current', 'upcoming', 'upcoming']);
    });

    it('giai đoạn cuối → mọi giai đoạn trước done, cuối current', () => {
      const result = stageChecklist(stages, 2, 19);
      expect(result.map((s) => s.status)).toEqual(['done', 'done', 'current']);
    });
  });
  ```

- [ ] **Bước 3: Chạy test, xác nhận fail** (2 hàm chưa tồn tại).

- [ ] **Bước 4: Cài đặt 2 hàm mới trong `seasonProgress.ts`**

  ```typescript
  export const remainingDays = (totalDays: number, elapsed: number): number =>
      Math.max(0, totalDays - elapsed);

  export type StageChecklistStatus = 'done' | 'current' | 'upcoming';

  export interface StageChecklistItem {
      name: string;
      status: StageChecklistStatus;
  }

  /**
   * Checklist giai đoạn cho zeigarnik-effect: mỗi mục chưa hoàn thành là 1
   * "open loop" — hiển thị rõ còn bao nhiêu giai đoạn nữa mới xong mùa vụ,
   * thay vì chỉ hiện tên giai đoạn hiện tại như UI cũ.
   */
  export const stageChecklist = (
      stages: CropStage[],
      currentStageIndex: number,
      _elapsed: number,
  ): StageChecklistItem[] =>
      stages.map((s, i) => ({
          name: s.name,
          status: i < currentStageIndex ? 'done' : i === currentStageIndex ? 'current' : 'upcoming',
      }));
  ```
  (Tham số `_elapsed` được giữ trong chữ ký để tương lai có thể tính "done" theo thời gian thực tế trôi qua thay vì chỉ theo `currentStageIndex` do backend báo — hiện tại backend là nguồn sự thật duy nhất cho giai đoạn hiện tại (`activeRecipe.current_stage_index`), nên hàm không tự suy luận lại từ elapsed để tránh 2 nguồn sự thật lệch nhau.)

- [ ] **Bước 5: Chạy lại, xác nhận pass** — tất cả PASS (cũ + mới).

- [ ] **Bước 6: Commit**

  ```bash
  git add hydragrow-frontend/src/lib/seasons/seasonProgress.ts \
          hydragrow-frontend/src/lib/seasons/seasonProgress.test.ts
  git commit -m "feat(seasons): add remainingDays + stageChecklist pure helpers (LAYER2-ZEIGARNIK-001)"
  ```

### Task 21: `SeasonStageChecklist` — checklist giai đoạn hiển thị trong `ActiveSeasonCard`

**Files:**
- Create: `hydragrow-frontend/src/components/seasons/SeasonStageChecklist.tsx`
- Create: `hydragrow-frontend/src/components/seasons/SeasonStageChecklist.test.tsx`
- Modify: `hydragrow-frontend/src/components/seasons/ActiveSeasonCard.tsx`

- [ ] **Bước 1: Viết test thất bại**

  ```tsx
  // hydragrow-frontend/src/components/seasons/SeasonStageChecklist.test.tsx
  import { render, screen } from '@testing-library/react';
  import { describe, expect, it } from 'vitest';
  import { SeasonStageChecklist } from './SeasonStageChecklist';

  describe('SeasonStageChecklist', () => {
    const items = [
      { name: 'Nảy mầm', status: 'done' as const },
      { name: 'Sinh trưởng', status: 'current' as const },
      { name: 'Ra hoa', status: 'upcoming' as const },
    ];

    it('render đủ tên 3 giai đoạn', () => {
      render(<SeasonStageChecklist items={items} remainingDaysCount={12} />);
      expect(screen.getByText('Nảy mầm')).toBeInTheDocument();
      expect(screen.getByText('Sinh trưởng')).toBeInTheDocument();
      expect(screen.getByText('Ra hoa')).toBeInTheDocument();
    });

    it('hiện số ngày còn lại — open loop có lối ra rõ ràng (zeigarnik-effect: "always provide a clear path to completion")', () => {
      render(<SeasonStageChecklist items={items} remainingDaysCount={12} />);
      expect(screen.getByText(/Còn 12 ngày/)).toBeInTheDocument();
    });

    it('giai đoạn done có icon check, current được nhấn mạnh', () => {
      render(<SeasonStageChecklist items={items} remainingDaysCount={12} />);
      const doneItem = screen.getByText('Nảy mầm').closest('li');
      const currentItem = screen.getByText('Sinh trưởng').closest('li');
      expect(doneItem?.className).toContain('text-faint');
      expect(currentItem?.className).toContain('text-primary-deep');
    });
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail.**

- [ ] **Bước 3: Cài đặt component**

  ```tsx
  // hydragrow-frontend/src/components/seasons/SeasonStageChecklist.tsx
  import { Check, Circle } from 'lucide-react';
  import type { StageChecklistItem } from '../../lib/seasons/seasonProgress';

  interface SeasonStageChecklistProps {
    items: StageChecklistItem[];
    remainingDaysCount: number;
  }

  /**
   * Checklist các giai đoạn mùa vụ — mỗi mục chưa "done" là 1 open loop
   * (zeigarnik-effect). Luôn kèm số ngày còn lại để open loop có lối ra rõ
   * ràng, tránh biến thành lo âu thay vì động lực (skill: "incomplete
   * without a route is anxiety, not motivation").
   */
  export const SeasonStageChecklist = ({ items, remainingDaysCount }: SeasonStageChecklistProps) => (
    <div className="space-y-2">
      <p className="text-[11px] font-semibold text-text-muted">
        Còn {remainingDaysCount} ngày để hoàn thành mùa vụ
      </p>
      <ul className="space-y-1.5">
        {items.map((item) => (
          <li
            key={item.name}
            className={`flex items-center gap-2 text-xs ${
              item.status === 'done'
                ? 'text-faint line-through'
                : item.status === 'current'
                ? 'text-primary-deep font-bold'
                : 'text-text-muted'
            }`}
          >
            {item.status === 'done' ? (
              <Check size={14} className="text-status shrink-0" />
            ) : (
              <Circle
                size={14}
                className={`shrink-0 ${item.status === 'current' ? 'text-primary fill-primary/20' : 'text-line'}`}
              />
            )}
            {item.name}
          </li>
        ))}
      </ul>
    </div>
  );
  ```

- [ ] **Bước 4: Chạy lại, xác nhận pass** — 3/3 PASS.

- [ ] **Bước 5: Gắn vào `ActiveSeasonCard.tsx`**

  Thêm import:
  ```typescript
  import { totalPlannedDays, elapsedDays, delayDays, remainingDays, stageChecklist } from '../../lib/seasons/seasonProgress';
  import { SeasonStageChecklist } from './SeasonStageChecklist';
  ```
  Sau dòng `const delay = activeRecipe ? delayDays(...) : 0;`, thêm:
  ```typescript
  const checklistItems = activeRecipe ? stageChecklist(activeRecipe.stages, activeRecipe.current_stage_index, elapsed) : [];
  const daysLeft = totalDays !== null ? Math.ceil(remainingDays(totalDays, elapsed)) : 0;
  ```
  Trong khối `{activeRecipe && totalDays !== null && (...)}` đã có progress bar, thêm `<SeasonStageChecklist>` ngay sau banner `delay > 0` (hoặc ngay sau progress bar nếu không có delay):
  ```tsx
  {activeRecipe && totalDays !== null && (
    <div className="space-y-2">
      <div className="flex items-center justify-between text-xs font-bold text-primary-deep">
        <span>Giai đoạn: {currentStage?.name || '—'} · Ngày {Math.floor(elapsed)}/{Math.round(totalDays)}</span>
      </div>
      <div className="h-2 bg-line rounded-full overflow-hidden">
        <div className="h-full bg-primary rounded-full transition-all" style={{ width: `${Math.min(100, (elapsed / totalDays) * 100)}%` }} />
      </div>
      {delay > 0 && (
        <Banner tone="warning" title={`Chậm hơn dự kiến ${Math.ceil(delay)} ngày`}>
          So với "{activeRecipe.recipe_id}" đang áp dụng
        </Banner>
      )}
      <SeasonStageChecklist items={checklistItems} remainingDaysCount={daysLeft} />
    </div>
  )}
  ```

- [ ] **Bước 6: Chạy test hiện có của `ActiveSeasonCard`, xác nhận không vỡ**

  ```bash
  npx vitest run src/components/seasons/ActiveSeasonCard.test.tsx src/components/seasons/SeasonStageChecklist.test.tsx
  npx tsc --noEmit
  ```

- [ ] **Bước 7: Commit**

  ```bash
  git add hydragrow-frontend/src/components/seasons/SeasonStageChecklist.tsx \
          hydragrow-frontend/src/components/seasons/SeasonStageChecklist.test.tsx \
          hydragrow-frontend/src/components/seasons/ActiveSeasonCard.tsx
  git commit -m "feat(seasons): stage checklist with remaining-days open loop on ActiveSeasonCard (LAYER2-ZEIGARNIK-002)"
  ```

### Task 22: `SeasonCompletionSummary` — màn tổng kết khi kết thúc mùa vụ (Peak-End)

**Files:**
- Create: `hydragrow-frontend/src/components/seasons/SeasonCompletionSummary.tsx`
- Create: `hydragrow-frontend/src/components/seasons/SeasonCompletionSummary.test.tsx`
- Modify: `hydragrow-frontend/src/components/seasons/ActiveSeasonCard.tsx`
- Modify: `hydragrow-frontend/src/pages/CropSeasons.tsx`

**Áp dụng peak-end-rule "Design the end deliberately... avoid ending on a confusing/administrative screen":** thay vì `window.confirm()` → quay thẳng về `CreateSeasonForm` trống, chèn 1 màn tổng kết (số ngày đã trồng, số ảnh nhật ký đã chụp) trước khi cho phép bắt đầu mùa mới — đây là "peak" tự nhiên của cả một chu kỳ trồng trọt kéo dài nhiều tuần, xứng đáng một khoảnh khắc ăn mừng thay vì biến mất ngay lập tức.

- [ ] **Bước 1: Viết test thất bại cho component tổng kết**

  ```tsx
  // hydragrow-frontend/src/components/seasons/SeasonCompletionSummary.test.tsx
  import { render, screen, fireEvent } from '@testing-library/react';
  import { describe, expect, it, vi } from 'vitest';
  import { SeasonCompletionSummary } from './SeasonCompletionSummary';

  describe('SeasonCompletionSummary', () => {
    it('hiển thị tên mùa vụ, số ngày trồng, số ảnh nhật ký', () => {
      render(
        <SeasonCompletionSummary
          seasonName="Vụ dâu tây Đông 2026"
          totalDaysGrown={42}
          photoCount={15}
          onClose={vi.fn()}
        />,
      );
      expect(screen.getByText('Vụ dâu tây Đông 2026')).toBeInTheDocument();
      expect(screen.getByText(/42 ngày/)).toBeInTheDocument();
      expect(screen.getByText(/15 ảnh/)).toBeInTheDocument();
    });

    it('bấm nút đóng gọi onClose', () => {
      const onClose = vi.fn();
      render(
        <SeasonCompletionSummary seasonName="Vụ A" totalDaysGrown={10} photoCount={0} onClose={onClose} />,
      );
      fireEvent.click(screen.getByText('Bắt đầu mùa vụ tiếp theo'));
      expect(onClose).toHaveBeenCalled();
    });

    it('photoCount=0 → vẫn hiển thị, không báo lỗi (không giả định luôn có ảnh)', () => {
      render(<SeasonCompletionSummary seasonName="Vụ A" totalDaysGrown={10} photoCount={0} onClose={vi.fn()} />);
      expect(screen.getByText(/0 ảnh/)).toBeInTheDocument();
    });
  });
  ```

- [ ] **Bước 2: Chạy test, xác nhận fail.**

- [ ] **Bước 3: Cài đặt component**

  ```tsx
  // hydragrow-frontend/src/components/seasons/SeasonCompletionSummary.tsx
  import { PartyPopper, Calendar, Image as ImageIcon, ArrowRight } from 'lucide-react';

  interface SeasonCompletionSummaryProps {
    seasonName: string;
    totalDaysGrown: number;
    photoCount: number;
    onClose: () => void;
  }

  /**
   * Màn "peak" khi kết thúc 1 mùa vụ — peak-end-rule: "the final moment of
   * a session shapes overall impression more than most of what preceded
   * it". Trước khi có component này, kết thúc mùa vụ chuyển thẳng về
   * CreateSeasonForm trống — đúng phản-pattern skill mô tả ("never end on
   * an administrative or transitional screen").
   */
  export const SeasonCompletionSummary = ({ seasonName, totalDaysGrown, photoCount, onClose }: SeasonCompletionSummaryProps) => (
    <div
      className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in"
      data-testid="season-completion-summary"
    >
      <div className="ui-card max-w-md w-full p-6 space-y-6 text-center shadow-2xl border border-line animate-in zoom-in-95">
        <div className="w-14 h-14 rounded-2xl bg-pill text-status flex items-center justify-center mx-auto">
          <PartyPopper size={32} />
        </div>
        <div className="space-y-1">
          <h2 className="text-xl font-bold text-primary-deep">Đã hoàn thành mùa vụ!</h2>
          <p className="text-base font-bold text-primary">{seasonName}</p>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div className="p-3 rounded-xl bg-surface-muted border border-line space-y-1">
            <Calendar size={18} className="text-primary mx-auto" />
            <p className="text-lg font-black text-primary-deep">{totalDaysGrown} ngày</p>
            <p className="text-[10px] text-text-muted uppercase tracking-wide">Thời gian canh tác</p>
          </div>
          <div className="p-3 rounded-xl bg-surface-muted border border-line space-y-1">
            <ImageIcon size={18} className="text-primary mx-auto" />
            <p className="text-lg font-black text-primary-deep">{photoCount} ảnh</p>
            <p className="text-[10px] text-text-muted uppercase tracking-wide">Nhật ký hình ảnh</p>
          </div>
        </div>

        <button
          type="button"
          onClick={onClose}
          className="w-full flex items-center justify-center gap-2 py-2.5 bg-primary hover:bg-primary-deep text-white rounded-lg font-bold text-sm transition-colors"
        >
          Bắt đầu mùa vụ tiếp theo <ArrowRight size={16} />
        </button>
      </div>
    </div>
  );
  ```

- [ ] **Bước 4: Chạy lại, xác nhận pass** — 3/3 PASS.

- [ ] **Bước 5: Nối luồng "vừa kết thúc mùa vụ" từ `ActiveSeasonCard` lên `CropSeasons.tsx`**

  Trong `ActiveSeasonCard.tsx`, thêm prop `onEnded?: (season: CropSeason, elapsedDaysGrown: number) => void`, gọi ngay sau `onEndSeason()` thành công trong `handleEnd`:
  ```typescript
  const handleEnd = async () => {
    if (window.confirm('Xác nhận kết thúc mùa vụ? Sau khi kết thúc, quy trình nuôi trồng trên trạm sẽ được hoàn tất và chuyển vào lịch sử.')) {
      await onEndSeason();
      onEnded?.(activeSeason, Math.floor(elapsed));
    }
  };
  ```
  Cập nhật interface:
  ```typescript
  interface ActiveSeasonCardProps {
    activeSeason: CropSeason;
    isLoading: boolean;
    onEndSeason: () => Promise<any>;
    onUpdateSeason?: (name: string, plantType: string, description: string) => Promise<any>;
    onEnded?: (season: CropSeason, elapsedDaysGrown: number) => void;
  }
  ```

  Trong `CropSeasons.tsx`, bắt sự kiện này và hiển thị summary trước khi `CreateSeasonForm` xuất hiện lại:
  ```tsx
  import { useState } from 'react';
  import { useQueryClient } from '@tanstack/react-query';
  import { useDeviceStore } from '../store/useDeviceStore';
  import { SeasonCompletionSummary } from '../components/seasons/SeasonCompletionSummary';
  import type { CropSeason } from '../types/models';

  export const CropSeasons = ({ variant = 'standalone' }: { variant?: 'standalone' | 'embedded' }) => {
    const { activeSeason, history, isLoading, createSeason, endSeason, updateSeason, deleteSeason } = useCropSeason();
    const deviceId = useDeviceStore((s) => s.deviceId);
    const queryClient = useQueryClient();
    const [justEnded, setJustEnded] = useState<{ season: CropSeason; days: number } | null>(null);

    if (isLoading && !activeSeason && history.length === 0) {
      return <LoadingState message="Đang tải danh sách mùa vụ..." />;
    }

    const filteredHistory = history.filter((season) => season.id !== activeSeason?.id);
    const photoCount = justEnded
      ? (queryClient.getQueryData<{ id: string }[]>(['season-photos', deviceId, justEnded.season.id]) ?? []).length
      : 0;

    const contentNode = (
      <>
        {activeSeason ? (
          <>
            <ActiveSeasonCard
              activeSeason={activeSeason}
              isLoading={isLoading}
              onEndSeason={endSeason}
              onUpdateSeason={updateSeason}
              onEnded={(season, days) => setJustEnded({ season, days })}
            />
            <SeasonPhotoJournal seasonId={activeSeason.id} seasonStartTime={activeSeason.start_time} />
          </>
        ) : (
          <CreateSeasonForm isLoading={isLoading} onCreateSeason={createSeason} />
        )}

        <SeasonHistoryList seasons={filteredHistory} onDelete={(season) => deleteSeason(season.id)} />

        {justEnded && (
          <SeasonCompletionSummary
            seasonName={justEnded.season.name}
            totalDaysGrown={justEnded.days}
            photoCount={photoCount}
            onClose={() => setJustEnded(null)}
          />
        )}
      </>
    );

    if (variant === 'embedded') return contentNode;

    return (
      <div className="p-4 md:p-8 max-w-4xl mx-auto pb-28">
        <PageHeader icon={Sprout} title="Quản Lý Mùa Vụ" subtitle="Theo dõi và ghi chép chu kỳ sinh trưởng của cây trồng" />
        {contentNode}
      </div>
    );
  };
  ```
  Ghi chú: `photoCount` đọc trực tiếp từ cache TanStack Query bằng đúng `queryKey` mà `SeasonPhotoJournal.tsx` đã dùng (`['season-photos', deviceId, seasonId]`) — không gọi thêm API nào, tận dụng dữ liệu đã fetch khi mùa vụ còn đang chạy. Nếu cache trống (chưa từng mở tab ảnh), `photoCount` hiển thị 0 — đúng và trung thực, không hiển thị sai số.

- [ ] **Bước 6: Chạy toàn bộ test của Track E**

  ```bash
  npx vitest run src/components/seasons/ src/pages/CropSeasons.test.tsx 2>/dev/null || npx vitest run src/components/seasons/
  npx tsc --noEmit
  ```
  (Kiểm tra `find hydragrow-frontend/src/pages -iname "CropSeasons.test*"` trước — nếu không tồn tại, lệnh fallback chạy đúng phần `components/seasons/` là đủ.)

- [ ] **Bước 7: Commit**

  ```bash
  git add hydragrow-frontend/src/components/seasons/SeasonCompletionSummary.tsx \
          hydragrow-frontend/src/components/seasons/SeasonCompletionSummary.test.tsx \
          hydragrow-frontend/src/components/seasons/ActiveSeasonCard.tsx \
          hydragrow-frontend/src/pages/CropSeasons.tsx
  git commit -m "feat(seasons): celebratory completion summary instead of ending on a blank create-form (LAYER2-PEAKEND-001)"
  ```

### Task 23: Khép guard cho `ActiveRecipeStatus.tsx` (được nhúng trong trang Seasons)

**Files:**
- Modify: `hydragrow-frontend/src/components/recipes/ActiveRecipeStatus.tsx`

- [ ] **Bước 1: Sửa dòng vi phạm**

  Dòng 54: `text-orange-700` → xem đúng ngữ cảnh dòng đó trước khi thay (khả năng cao là badge cảnh báo "chậm tiến độ"/giai đoạn — nếu đúng vậy, dùng `text-warn-deep` sẵn có, khớp đúng ngữ nghĩa "cảnh báo nhẹ" thay vì 1 hue cam rời rạc không có trong bất kỳ theme nào đã định nghĩa):
  ```bash
  sed -n '45,60p' hydragrow-frontend/src/components/recipes/ActiveRecipeStatus.tsx
  ```
  Thay `text-orange-700` bằng `text-warn-deep` nếu ngữ cảnh xác nhận đây là trạng thái cảnh báo; nếu ngữ cảnh thực tế khác (ví dụ đang biểu diễn 1 giai đoạn cụ thể trong danh mục giai đoạn, không phải cảnh báo), dùng `text-primary-deep` thay vào đó — quyết định dựa trên đọc code thật tại bước này, không đoán trước.

- [ ] **Bước 2: Chạy guard**

  ```bash
  npx vitest run src/lib/design-lint/hardcodedColors.test.ts
  npx tsc --noEmit
  ```
  Kỳ vọng: guard chỉ còn đúng 3 file tracked-debt từ Task 0 — **toàn bộ 5 track A–E đã khép guard cho phần của mình**, khớp với những gì Task 0 dự kiến.

- [ ] **Bước 3: Commit**

  ```bash
  git add hydragrow-frontend/src/components/recipes/ActiveRecipeStatus.tsx
  git commit -m "fix(recipes): ActiveRecipeStatus hardcoded orange -> semantic token (LAYER2-PEAKEND-002)"
  ```

---

## Track F — Tích hợp & tài liệu hoá (sau khi A–E đã merge)

### Task 24: Cập nhật `docs/design-system/PATTERN-LIBRARY.md` — mục 6, 7

**Files:**
- Modify: `docs/design-system/PATTERN-LIBRARY.md`

- [ ] **Bước 1: Thêm mục 6 — `DosingHourlyChart` / `Sparkline`**

  ```markdown

  ---

  ## 6. `DosingHourlyChart` / `Sparkline` — data-visualization dùng chung

  **Files:** `hydragrow-frontend/src/components/dosing/DosingHourlyChart.tsx`,
  `hydragrow-frontend/src/components/ui/Sparkline.tsx`.

  `DosingHourlyChart` vẽ cột nhóm 24 giờ × 4 bơm, màu lấy từ
  `pumpVisualTheme.ts` (mục 5), luôn kèm bảng `sr-only` làm text alternative
  và `aria-label` theo từng cột giờ. `Sparkline` là đường xu hướng SVG tối
  giản cho 1 chuỗi số — dùng cho chỉ số không có endpoint lịch sử ở backend
  (xem `useHealthHistory.ts`: ring-buffer trong bộ nhớ phiên, không phải
  lịch sử vĩnh viễn).

  **Don't:** tự vẽ lại 1 dãy `<div style={{height}}>` mới cho biểu đồ cột —
  đó chính là cách `DosingTotalCard.tsx` từng làm trước khi có
  `DosingHourlyChart` (không nhãn trục, không chú giải, không text
  alternative). **Dùng `DosingHourlyChart` cho dữ liệu nhiều-chuỗi-theo-giờ,
  `Sparkline` cho 1 chuỗi xu hướng đơn giản.**

  ---

  ## 7. `pumpControlStateMachine` — trạng thái điều khiển thiết bị 3 chế độ

  **File:** `hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.ts`

  `derivePumpControlState({currentStatus, isAutoMode, isEmergency,
  lockedByPumpId})` → `{state: 'idle'|'running'|'locked', reason:
  'auto_mode'|'emergency'|'interlock'|null}`. `running` luôn thắng mọi lý do
  khoá (bơm đang thực sự chạy thì không có ý nghĩa hiển thị "đã khoá" cho
  lệnh BẬT tiếp theo). Hiển thị qua `PumpControlStatePill` (mục 5); lý do
  khoá `interlock` còn được hiển thị đầy đủ qua `Banner` (xem
  `AdvancedDeviceControl.tsx`).

  **Don't:** viết lại biểu thức `isAutoMode || (isEmergency && !currentStatus)
  || Boolean(lockedByPumpId)` ở component khác. **Import
  `derivePumpControlState` thay vào đó** — logic ưu tiên giữa 3 lý do khoá
  (`auto_mode > emergency > interlock`) chỉ nên tồn tại ở một nơi.
  ```

- [ ] **Bước 2: Commit**

  ```bash
  git add docs/design-system/PATTERN-LIBRARY.md
  git commit -m "docs(design-system): document DosingHourlyChart/Sparkline/pumpControlStateMachine patterns (LAYER2-INTEGRATION-001)"
  ```

### Task 25: Cập nhật `DESIGN.md` §15 và `CHUAN-GIAO-DIEN-FRONTEND.md`

**Files:**
- Modify: `hydragrow-frontend/DESIGN.md`
- Modify: `hydragrow-frontend/CHUAN-GIAO-DIEN-FRONTEND.md`

- [ ] **Bước 1: Thêm dòng vào `DESIGN.md` §15 (Technical Debt)**

  ```markdown
  - **`SensorBentoCard.tsx`, `ConfigBackup.tsx`, `RecipeBuilder.tsx`** vẫn
    dùng màu Tailwind hardcode ngoài token — khoanh vùng có chủ đích trong
    Layer 2 rollout (2026-09-13), không phải bị bỏ sót; xem
    `colorAllowlist.json` mục `_comment_trackedDebt` và
    `docs/design-system/PR-QA-CHECKLIST.md` mục 9.
  ```

- [ ] **Bước 2: Cập nhật `CHUAN-GIAO-DIEN-FRONTEND.md` nếu file này liệt kê danh sách component dùng chung (kiểm tra trước khi sửa)**

  ```bash
  grep -n "DeviceStatePill\|StatusPill\|Banner" hydragrow-frontend/CHUAN-GIAO-DIEN-FRONTEND.md
  ```
  Nếu có mục liệt kê tương tự PATTERN-LIBRARY.md, thêm đúng 2 dòng tham chiếu tới `PumpControlStatePill` và `DosingHourlyChart` theo đúng định dạng đang có trong file (không đoán định dạng — đọc file thật rồi mới thêm cho khớp).

- [ ] **Bước 3: Commit**

  ```bash
  git add hydragrow-frontend/DESIGN.md hydragrow-frontend/CHUAN-GIAO-DIEN-FRONTEND.md
  git commit -m "docs(frontend): sync DESIGN.md tracked debt + CHUAN-GIAO component list (LAYER2-INTEGRATION-002)"
  ```

### Task 26: Xác minh toàn bộ (full-suite) — điều kiện đóng plan

**Files:** không sửa file — chỉ chạy lệnh và xác nhận.

- [ ] **Bước 1: Type-check + lint toàn bộ**

  ```bash
  cd hydragrow-frontend
  npx tsc --noEmit
  npx eslint .
  ```
  Kỳ vọng: cả hai sạch, 0 lỗi.

- [ ] **Bước 2: Toàn bộ test suite**

  ```bash
  npx vitest run
  ```
  Kỳ vọng: 100% PASS.

- [ ] **Bước 3: Guard màu — xác nhận đúng đường ranh đã vẽ ở Task 0**

  ```bash
  npx vitest run src/lib/design-lint/hardcodedColors.test.ts
  ```
  Kỳ vọng: **vẫn FAIL, đúng và chỉ đúng 3 file tracked-debt** (`SensorBentoCard.tsx`, `ConfigBackup.tsx`, `RecipeBuilder.tsx`). Nếu danh sách vi phạm có bất kỳ file nào khác 3 file này — một trong các Track A–E chưa khép guard đúng như task của nó yêu cầu; quay lại đúng task tương ứng, không sửa Task 0 để "giấu" vi phạm mới.

- [ ] **Bước 4: Phía Tauri desktop (không track nào ở trên chạm vào, nhưng bắt buộc theo DESIGN.md §13)**

  ```bash
  cd src-tauri && cargo check && cargo clippy -- -D warnings
  ```

- [ ] **Bước 5: Đối chiếu checklist tự-rà (Self-Review, Section 4 của plan này) trước khi coi Layer 2 là "done"**

  Không có bước code ở đây — đọc Section 4 bên dưới và xác nhận từng mục.

- [ ] **Bước 6: Commit cuối (nếu Bước 1-4 có phát sinh sửa nhỏ)**

  ```bash
  git add -A
  git commit -m "chore: final Layer 2 rollout verification pass (LAYER2-INTEGRATION-003)"
  ```

---

## 4. Self-Review

**1. Spec coverage — đối chiếu lại 6 hạng mục gốc:**

| Hạng mục gốc | Track | Đã tự hỏi "quy mô lớn hơn nghĩa là gì" |
|---|---|---|
| `state-machine` (pill 3 trạng thái dosing) | A, Task 1-2 | Không chỉ vẽ pill — có state machine thuần + test riêng, sửa luôn bug `colorTheme="purple"` phát hiện khi đọc code |
| `error-handling-ux` (banner/lock note) | A, Task 4-6 | Áp cả 4 tầng Prevention/Detection/Communication/Recovery, không chỉ đổi màu 1 div |
| `data-visualization` (D16, D18) | B, Task 7-12 | Thêm dữ liệu theo-từng-bơm (trước đây gộp lẫn), thêm sparkline cho Analytics vốn hoàn toàn không có chart trong app |
| `form-design` (D9 Pairing) | C, Task 13-16 | Validate on-blur + error placement đúng field + lối thoát "quét lại" khi mã không khớp |
| `search-ux` (Journal) | D, Task 17-19 | Nút xoá + đếm kết quả + zero-state nhắc lại từ khoá + highlight kết quả |
| `zeigarnik-effect`/`peak-end-rule` (Seasons) | E, Task 20-23 | Checklist giai đoạn (open loop có lối ra) + màn tổng kết khi kết thúc mùa (peak thật, không kết ở màn trống) |

**2. Placeholder scan:** không có `TODO`/`implement later`/"tương tự Task N" trong bất kỳ bước nào — mọi bước code đều là code đầy đủ, copy-paste được. Các bước "đọc file trước khi sửa" (ví dụ Task 19 Bước 9, Task 23 Bước 1) được viết tường minh là bước cần đọc code thật để chọn đúng nhánh, không phải chỗ trống che giấu quyết định chưa đưa ra.

**3. Type consistency:** `PumpThemeKey` (Task 3) được dùng nhất quán ở `AdvancedDeviceControl.tsx`, `ControlPanel.tsx`, `DosingHourlyChart.tsx`, `DosingReportCard.tsx` — không có nơi nào tự khai lại union type. `PumpControlState`/`PumpLockReason` (Task 1) chỉ được định nghĩa 1 lần, dùng lại ở `PumpControlStatePill` (Task 2) và `AdvancedDeviceControl` (Task 4). `StageChecklistItem` (Task 20) dùng lại nguyên vẹn ở `SeasonStageChecklist` (Task 21).

**4. Guard màu — điều kiện đóng plan:** sau Task 26, `hardcodedColors.test.ts` fail đúng và chỉ 3 file đã khoanh vùng ở Task 0. Đây là tiêu chí khách quan, chạy được, để biết Layer 2 rollout đã "xong" theo đúng nghĩa guard-based mà GOVERNANCE.md đòi hỏi — không dựa vào cảm tính "trông ổn".

---

## 5. Execution Handoff

**Plan đã lưu tại `docs/superpowers/plans/2026-09-13-layer2-design-ux-agentteams-rollout.md`.** Hai lựa chọn thực thi:

**1. Subagent-Driven (khuyến nghị trong Claude)** — dùng `superpowers:subagent-driven-development`: dispatch Task 0 trước (baseline), sau đó 5 subagent song song cho Track A–E (mỗi subagent nhận đúng "File perimeter" đã ghi trong roster YAML ở Section 1, dù không chạy qua DSH), review giữa các task, cuối cùng 1 phiên chạy Track F.

**2. dsh-agent-teams (nếu đang chạy DeepSeek Harness)** — dán YAML ở Section 1 vào `cordis.patch.yml`, gọi `/agent-teams --profile hydragrow-layer2 ...`, Approve & Run. DAG đã cố định (`taskPlanning: seed`) đúng theo Section 3 của plan này — Captain không tự vẽ lại phạm vi.

**3. Inline Execution (tuần tự trong 1 phiên)** — dùng `superpowers:executing-plans`, chạy Task 0 → A → B → C → D → E → F theo đúng thứ tự, batch theo từng Task, checkpoint sau mỗi Track.

**Bạn muốn tôi tiếp tục theo hướng nào?**
