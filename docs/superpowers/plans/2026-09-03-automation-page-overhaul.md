# Automation Page Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the HYDRAGROW Automation page into one coherent, production-grade React Flow experience matching the supplied Phase 0–7 reference screens, while preserving the already-implemented automation IR/backend behavior.

**Architecture:** Keep React Flow as the only desktop editor. Make the page shell, overview canvas, editor drawer, trigger/condition/action editors, dry-run panel, chain selector, and multi-device template UI share the existing `App.css` design tokens. Extract the shared drawer chrome before parallel work so later lanes do not collide on `FlowDetailDrawer.tsx`; keep backend feature logic intact except for verification/fixes proven necessary by integration tests.

**Tech Stack:** React 19, TypeScript 5.8, Vite, `@xyflow/react`, TanStack Query, Zod, Tailwind CSS v4, Lucide, `react-hot-toast`; backend Rust/Actix/sqlx/Rhai/InfluxDB only where UI integration exposes a real defect.

---

## Source of truth

The supplied references are the visual target: `00` through `09` screenshots. The project UI standard already mandates `Inter`, emerald/amber/red/sky token usage, `.app-page`, `.ui-card`, shared input/button classes, desktop sidebar behavior, mobile flow cards, and React Flow as the single desktop automation editor. The current `Automation.tsx` still uses a bare React Flow canvas and drawer layout, while `FlowDetailDrawer.tsx` mixes header, editor, next-flow selection, test panel, and persistence in one component. The current `useAutomationBuilder.ts` seeds a fixed trigger → condition → action graph and supports graph persistence, while `ir.ts` already contains condition groups, time-window modes, action variants, webhook config, and `next_flow_ids`. See `hydragrow-frontend/CHUAN-GIAO-DIEN-FRONTEND.md`, `Automation.tsx`, `FlowDetailDrawer.tsx`, `useAutomationBuilder.ts`, and `ir.ts`.

The repository history shows AUTOMATION-004 through AUTOMATION-008 were implemented recently: time-window conditions, cron, webhook binding, dry-run test panel, and multi-device templates. Treat those backend contracts as existing; this plan focuses on making the page visually coherent and ensuring each existing contract is surfaced correctly in UI. Where the checked-out `main` source disagrees with the corresponding acceptance/history claim, verify the actual file on the implementation branch before coding instead of assuming either side is correct.

---

## Task 1: Establish Automation UI foundation and isolate shared drawer chrome

**Files:**
- Create: `hydragrow-frontend/src/components/automation/AutomationPageHeader.tsx`
- Create: `hydragrow-frontend/src/components/automation/FlowEditorHeader.tsx`
- Create: `hydragrow-frontend/src/components/automation/FlowEditorFooter.tsx`
- Create: `hydragrow-frontend/src/components/automation/NextFlowSelector.tsx`
- Modify: `hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx`
- Modify: `hydragrow-frontend/src/App.css`
- Test: `hydragrow-frontend/src/components/automation/FlowDetailDrawer.test.tsx`
- Test: `hydragrow-frontend/src/components/automation/NextFlowSelector.test.tsx`

- [ ] **Step 1: Write failing tests for shared drawer boundaries**

Add tests proving:

```tsx
it('renders editor header with name, kind and enabled control', () => {
  render(<FlowEditorHeader name="pH quá cao" kind="alert" enabled onChange={vi.fn()} />);
  expect(screen.getByDisplayValue('pH quá cao')).toBeInTheDocument();
  expect(screen.getByText('Alert')).toBeInTheDocument();
  expect(screen.getByRole('checkbox')).toBeChecked();
});

it('renders selected next flows and cycle warning', () => {
  render(
    <NextFlowSelector
      scripts={[candidate, cyclicCandidate]}
      selectedIds={[candidate.id]}
      onToggle={vi.fn()}
      isCycle={(id) => id === cyclicCandidate.id}
    />,
  );
  expect(screen.getByText(candidate.name)).toBeInTheDocument();
  expect(screen.getByText('không cho phép — sẽ tạo vòng lặp')).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused tests and verify they fail**

Run:
```bash
cd hydragrow-frontend
npm run test -- src/components/automation/FlowDetailDrawer.test.tsx src/components/automation/NextFlowSelector.test.tsx
```
Expected: FAIL because the extracted components do not yet exist.

- [ ] **Step 3: Implement shared components using existing tokens**

`FlowEditorHeader.tsx` owns only name/kind/enabled controls. `FlowEditorFooter.tsx` owns delete/test/save affordances. `NextFlowSelector.tsx` owns candidate listing, checked state, cycle-disabled state, and helper text. Use `.ui-input`, `.ui-btn-md`, `.ui-btn-primary`, `.ui-card`, `.farm-muted-panel`, and emerald/amber/red tokens only.

`AutomationPageHeader.tsx` must render the exact page anatomy required by the UI standard:

```tsx
<div className="page-header">
  <div>
    <h1 className="page-header-title">Tự động hóa</h1>
    <p className="page-header-subtitle">
      Thiết kế các luồng điều khiển bơm, van và cảm biến cho trạm.
    </p>
  </div>
  <button className="ui-btn-primary" onClick={onNewFlow}>+ Flow mới</button>
</div>
```

Add a `ui-btn-primary` definition to `App.css` only if it is still missing in the checked-out branch:

```css
.ui-btn-primary {
  @apply inline-flex items-center justify-center rounded-xl bg-emerald-700 px-4 py-2 text-sm font-semibold text-white transition-colors hover:bg-emerald-800 disabled:cursor-not-allowed disabled:opacity-50;
}
```

- [ ] **Step 4: Refactor `FlowDetailDrawer.tsx` to compose the extracted pieces**

Keep all existing persistence calls, IR validation, `compileToRhai`, and cycle detection behavior. Replace inline header/footer/next-flow markup with the new components. Do not change request payload shapes.

- [ ] **Step 5: Run tests and lint**

Run:
```bash
cd hydragrow-frontend
npm run test -- src/components/automation/FlowDetailDrawer.test.tsx src/components/automation/NextFlowSelector.test.tsx
npm run lint
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add hydragrow-frontend/src/components/automation/AutomationPageHeader.tsx hydragrow-frontend/src/components/automation/FlowEditorHeader.tsx hydragrow-frontend/src/components/automation/FlowEditorFooter.tsx hydragrow-frontend/src/components/automation/NextFlowSelector.tsx hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx hydragrow-frontend/src/App.css hydragrow-frontend/src/components/automation/FlowDetailDrawer.test.tsx hydragrow-frontend/src/components/automation/NextFlowSelector.test.tsx
git commit -m "refactor(automation): isolate shared editor chrome"
```

---

## Task 2: Rebuild Automation overview canvas to match reference `01`

**Files:**
- Modify: `hydragrow-frontend/src/pages/Automation.tsx`
- Modify: `hydragrow-frontend/src/hooks/useFlowCanvas.ts`
- Modify: `hydragrow-frontend/src/components/automation/reactflow/FlowSummaryNode.tsx`
- Create: `hydragrow-frontend/src/pages/Automation.test.tsx`
- Test: `hydragrow-frontend/src/hooks/useFlowCanvas.test.tsx`

- [ ] **Step 1: Write failing tests for overview states and chain edges**

```tsx
it('shows empty-state card when no flows exist', () => {
  render(<Automation variant="embedded" flow={emptyFlowMock} scripts={[]} />);
  expect(screen.getByText('Chưa có Flow nào')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: /Tạo Flow mới/i })).toBeInTheDocument();
});

it('builds animated edges from next_flow_ids', () => {
  const { result } = renderHook(() => useFlowCanvas([flowA, flowB]));
  expect(result.current.edges).toHaveLength(1);
  expect(result.current.edges[0]).toMatchObject({ source: flowA.id, target: flowB.id, animated: true });
});
```

- [ ] **Step 2: Run failing tests**

Run:
```bash
cd hydragrow-frontend
npm run test -- src/pages/Automation.test.tsx src/hooks/useFlowCanvas.test.tsx
```
Expected: FAIL if the current branch still has `edges: []` in `useFlowCanvas`.

- [ ] **Step 3: Implement overview data model**

For each saved script, expose:
- kind badge (`ALERT`, `ACTION`, `RECIPE`)
- trigger badge (`CRON`, `WEBHOOK`, or default sensor/FSM)
- condition/action summary counts
- disabled state
- selected/open state
- outgoing chain edges from `ir_json.next_flow_ids`

Build edges as green dashed animated edges with stable IDs, preserving source/target IDs from the saved flow graph.

- [ ] **Step 4: Redesign `FlowSummaryNode.tsx`**

Match reference `01`: compact white card, rounded border, kind badge at top right, condition/action count line, trigger badge at bottom, muted styling for disabled flow. Use only design tokens from the UI standard. Do not use `text-gray-*`, `slate-*`, `blue-*`, or arbitrary hex values.

- [ ] **Step 5: Rewrite `Automation.tsx` layout**

Desktop (`lg:`):
- `.app-page`
- `AutomationPageHeader`
- single `.ui-card` containing overview canvas + optional editor drawer
- React Flow gets explicit min height and responsive width
- `fitView` with deterministic padding

Mobile/tablet: render flow cards/list instead of embedding a draggable canvas, with a single-column editor route/drawer behavior.

Add loading and error states using existing shared components where available; do not invent a second loading/error system.

- [ ] **Step 6: Run tests/build**

```bash
cd hydragrow-frontend
npm run test -- src/pages/Automation.test.tsx src/hooks/useFlowCanvas.test.tsx
npm run build
```
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add hydragrow-frontend/src/pages/Automation.tsx hydragrow-frontend/src/hooks/useFlowCanvas.ts hydragrow-frontend/src/components/automation/reactflow/FlowSummaryNode.tsx hydragrow-frontend/src/pages/Automation.test.tsx hydragrow-frontend/src/hooks/useFlowCanvas.test.tsx
git commit -m "feat(automation): rebuild overview canvas and responsive flow list"
```

---

## Task 3: Redesign node palette and editor composition to match reference `02`

**Files:**
- Modify: `hydragrow-frontend/src/components/automation/reactflow/NodePalette.tsx`
- Modify: `hydragrow-frontend/src/hooks/useAutomationBuilder.ts`
- Modify: `hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx`
- Modify: `hydragrow-frontend/src/components/automation/reactflow/nodeTypes.ts`
- Create: `hydragrow-frontend/src/components/automation/reactflow/NodePalette.test.tsx`
- Test: `hydragrow-frontend/src/hooks/useAutomationBuilder.test.ts`

- [ ] **Step 1: Write failing palette tests**

```tsx
it('shows trigger, condition, delay and action groups', () => {
  render(<NodePalette onAddNode={vi.fn()} />);
  expect(screen.getByText('+ Sensor')).toBeInTheDocument();
  expect(screen.getByText('+ FSM giai đoạn')).toBeInTheDocument();
  expect(screen.getByText('+ Cron (lịch)')).toBeInTheDocument();
  expect(screen.getByText('+ Webhook')).toBeInTheDocument();
  expect(screen.getByText('+ Condition')).toBeInTheDocument();
  expect(screen.getByText('+ Condition Group (AND/OR)')).toBeInTheDocument();
  expect(screen.getByText('+ Time-window (mean/min/max)')).toBeInTheDocument();
  expect(screen.getByText('+ Delay')).toBeInTheDocument();
  expect(screen.getByText('+ Alert')).toBeInTheDocument();
  expect(screen.getByText('+ Dose / Water / Emergency stop')).toBeInTheDocument();
  expect(screen.getByText('+ Advance stage / End season')).toBeInTheDocument();
  expect(screen.getByText('+ Chain → chạy Flow khác')).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused test**

Expected: FAIL with missing palette labels.

- [ ] **Step 3: Expand palette API without duplicating business logic**

Keep one callback contract for node insertion. Add metadata per button (`group`, `label`, `tone`, `new`) so the palette can render the reference grouping. Existing `condition_group` and `action` behavior stays unchanged.

- [ ] **Step 4: Update builder insertion rules**

Add dedicated node types only where the existing IR requires them. Do not invent new persisted IR fields for visual-only nodes. Ensure adding a node updates React Flow state and preserves `buildIrFromGraph` round-tripping.

- [ ] **Step 5: Style and test**

Match `02`: compact grouped pills; new capabilities use the teal/blue/green accents already defined in the supplied legend, while action/condition colors stay consistent with existing semantics.

Run:
```bash
cd hydragrow-frontend
npm run test -- src/components/automation/reactflow/NodePalette.test.tsx src/hooks/useAutomationBuilder.test.ts
npm run lint
```

- [ ] **Step 6: Commit**

```bash
git add hydragrow-frontend/src/components/automation/reactflow/NodePalette.tsx hydragrow-frontend/src/hooks/useAutomationBuilder.ts hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx hydragrow-frontend/src/components/automation/reactflow/nodeTypes.ts hydragrow-frontend/src/components/automation/reactflow/NodePalette.test.tsx hydragrow-frontend/src/hooks/useAutomationBuilder.test.ts
git commit -m "feat(automation): expand node palette and editor affordances"
```

---

## Task 4: Rebuild condition group editor to match reference `03`

**Files:**
- Modify: `hydragrow-frontend/src/components/automation/reactflow/ConditionGroupEditor.tsx`
- Modify: `hydragrow-frontend/src/lib/automation/conditionTree.ts`
- Test: `hydragrow-frontend/src/components/automation/reactflow/ConditionGroupEditor.test.tsx`
- Test: `hydragrow-frontend/src/lib/automation/conditionTree.test.ts`

- [ ] **Step 1: Add failing tests for nested AND/OR semantics**

```tsx
it('renders root AND/OR switch and nested group controls', () => {
  render(<ConditionGroupEditor group={{ op: 'and', children: [
    { op: 'or', children: [
      { sensor: 'ph', operator: '<', value: 5.5 },
      { sensor: 'ph', operator: '>', value: 7.5 },
    ] },
    { sensor: 'ec', operator: '>', value: 3.0 },
  ]}} fields={['ph', 'ec']} onChange={vi.fn()} isRoot />);
  expect(screen.getByRole('button', { name: 'AND' })).toHaveAttribute('aria-pressed', 'true');
  expect(screen.getByText('Nhóm con')).toBeInTheDocument();
  expect(screen.getAllByRole('button', { name: 'OR' }).length).toBeGreaterThan(0);
});
```

- [ ] **Step 2: Run focused tests and verify baseline failures for the new DOM shape**

- [ ] **Step 3: Implement visual tree editor**

Match `03`: root green dashed container, nested orange dashed container, compact leaf rows, visible remove button, AND/OR segmented controls, explicit `+ Thêm điều kiện` and `+ Thêm nhóm con (AND/OR)`, and generated-expression preview below the tree.

- [ ] **Step 4: Keep IR semantics lossless**

`toEditorRoot` and `fromEditorRoot` must continue to preserve every leaf, operator, nesting level, and group operator. Empty groups remain invalid at save time through existing Zod validation; editor may temporarily display an empty group while user is editing.

- [ ] **Step 5: Run test/build**

```bash
cd hydragrow-frontend
npm run test -- src/components/automation/reactflow/ConditionGroupEditor.test.tsx src/lib/automation/conditionTree.test.ts
npm run build
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add hydragrow-frontend/src/components/automation/reactflow/ConditionGroupEditor.tsx hydragrow-frontend/src/lib/automation/conditionTree.ts hydragrow-frontend/src/components/automation/reactflow/ConditionGroupEditor.test.tsx hydragrow-frontend/src/lib/automation/conditionTree.test.ts
git commit -m "feat(automation): redesign nested condition group editor"
```

---

## Task 5: Surface time-window conditions exactly like reference `04`

**Files:**
- Modify: `hydragrow-frontend/src/components/automation/reactflow/ConditionGroupEditor.tsx`
- Modify: `hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.tsx`
- Modify: `hydragrow-frontend/src/lib/automation/compileToRhai.ts` only if verification shows a real current mismatch
- Test: `hydragrow-frontend/src/components/automation/reactflow/ConditionGroupEditor.test.tsx`
- Test: `hydragrow-frontend/src/lib/automation/compileToRhai.test.ts`
- Backend verification only: `hydragrow-backend/src/db/influx.rs`, `hydragrow-backend/src/mqtt/handlers/script_eval.rs`

- [ ] **Step 1: Lock the UI contract with tests**

```tsx
it('switches a leaf to mean mode and edits window in minutes', () => {
  const onChange = vi.fn();
  render(<ConditionGroupEditor group={{ op: 'and', children: [
    { sensor: 'ph', operator: '>', value: 6.5 }
  ]}} fields={['ph']} onChange={onChange} isRoot />);
  fireEvent.change(screen.getByRole('combobox', { name: 'Chế độ đọc' }), { target: { value: 'mean' } });
  fireEvent.change(screen.getByRole('spinbutton', { name: 'Cửa sổ (phút)' }), { target: { value: '15' } });
  expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({
    children: [expect.objectContaining({ mode: 'mean', windowSec: 900 })],
  }));
});
```

- [ ] **Step 2: Verify compiler contract before changing code**

Run:
```bash
cd hydragrow-frontend
npm run test -- src/lib/automation/compileToRhai.test.ts
```
Expected: Existing `fetch_range_stat(sensor, mode, windowSec)` cases stay PASS.

- [ ] **Step 3: Implement reference `04` presentation**

Show a single condition row with read mode pills/select, window in minutes, operator/value, and a visual node summary such as `ph.mean(15m) > 6.5` with a `15m` badge.

- [ ] **Step 4: Verify backend integration instead of reimplementing it**

Run the existing backend tests:
```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml stat_to_flux_fn
cargo test --manifest-path hydragrow-backend/Cargo.toml range_stat_prefetch_tests
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-frontend/src/components/automation/reactflow/ConditionGroupEditor.tsx hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.tsx hydragrow-frontend/src/components/automation/reactflow/ConditionGroupEditor.test.tsx hydragrow-frontend/src/lib/automation/compileToRhai.test.ts
git commit -m "feat(automation): polish time-window condition editor"
```

---

## Task 6: Replace trigger placeholders with Cron UI matching reference `05`

**Files:**
- Modify: `hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.tsx`
- Create or restore: `hydragrow-frontend/src/hooks/useCronPreview.ts`
- Create or restore: `hydragrow-frontend/src/hooks/useCronPreview.test.ts`
- Modify: `hydragrow-frontend/src/lib/automation/ir.ts`
- Test: `hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.test.tsx`
- Backend verification: `hydragrow-backend/src/api/cron.rs`, `hydragrow-backend/src/services/cron_schedule.rs` (exact current paths must be confirmed on implementation branch)

- [ ] **Step 1: Write failing Cron UI tests**

```tsx
it('renders Cron configuration instead of phase placeholder', () => {
  render(<NodeEditorPanel kind="alert" node={{ id: 'trigger', type: 'trigger', data: { trigger: { type: 'cron', cron: '0 7 * * *', timezone: 'Asia/Ho_Chi_Minh' } } }} onChange={vi.fn()} onClose={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: 'Cron' }));
  expect(screen.getByText('LỊCH DỤNG SẴN')).toBeInTheDocument();
  expect(screen.getByText('07:00')).toBeInTheDocument();
  expect(screen.getByText('BIỂU THỨC CRON SINH RA')).toBeInTheDocument();
});
```

- [ ] **Step 2: Run failing test**

Expected: FAIL while Cron is still shown as `Sắp có ở Phase 4/5`.

- [ ] **Step 3: Implement Cron editor state**

Support four presets matching `05`:
- Every day at HH:MM
- Every hour
- Every week on selected weekday/time
- Custom five-field cron expression

Show generated five-field expression and next-run preview. Timezone defaults to the device station timezone already used elsewhere (`Asia/Ho_Chi_Minh` in the supplied reference).

- [ ] **Step 4: Implement `useCronPreview` against existing endpoint contract**

Do not add a second endpoint. Query the existing `GET /cron-preview` contract and map `{ cron, timezone }` to `{ nextRun }` UI data. Reject six-field user input in the frontend because the backend contract is five-field input.

- [ ] **Step 5: Run tests**

```bash
cd hydragrow-frontend
npm run test -- src/components/automation/reactflow/NodeEditorPanel.test.tsx src/hooks/useCronPreview.test.ts
```
Expected: PASS.

- [ ] **Step 6: Verify backend schedule tests**

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml cron_schedule
cargo test --manifest-path hydragrow-backend/Cargo.toml cron_preview
```
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.tsx hydragrow-frontend/src/hooks/useCronPreview.ts hydragrow-frontend/src/hooks/useCronPreview.test.ts hydragrow-frontend/src/lib/automation/ir.ts hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.test.tsx
git commit -m "feat(automation): add production Cron trigger editor"
```

---

## Task 7: Polish Webhook trigger binding to match reference `06`

**Files:**
- Modify: `hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.tsx`
- Modify: `hydragrow-frontend/src/components/automation/reactflow/WebhookFieldMappingEditor.tsx`
- Modify: `hydragrow-frontend/src/lib/automation/ir.ts` only if UI finds an uncovered persisted field
- Test: `hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.test.tsx`
- Test: `hydragrow-frontend/src/components/automation/reactflow/WebhookFieldMappingEditor.test.tsx`
- Backend verification: `hydragrow-backend/src/api/webhook.rs`, `hydragrow-backend/src/services/script_engine.rs`, `hydragrow-backend/src/mqtt/handlers/script_eval.rs`

- [ ] **Step 1: Write failing tests for the reference layout**

```tsx
it('shows webhook URL, processing mode, payload mapping and flow consumer', () => {
  render(<NodeEditorPanel kind="alert" node={{ id: 'trigger', type: 'trigger', data: { trigger: { type: 'webhook', mode: 'flow', fieldMappings: [] } } }} onChange={vi.fn()} onClose={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: 'Webhook' }));
  expect(screen.getByText('WEBHOOK URL')).toBeInTheDocument();
  expect(screen.getByText('CHẾ ĐỘ XỬ LÝ')).toBeInTheDocument();
  expect(screen.getByText('body.external_alarm')).toBeInTheDocument();
});
```

- [ ] **Step 2: Implement the reference `06` sections**

Show:
- URL with truncated token-safe preview
- two processing-mode choices (`Chạy qua Flow`, `Gọi lệnh trực tiếp`)
- field mapping rows as `body.path → target field`
- flow usage summary at bottom

Keep the existing `WebhookTriggerConfig` shape and mapping behavior.

- [ ] **Step 3: Add accessible mapping controls**

Use explicit labels/`aria-label`s for body path, target field, mode, add/remove mapping. Preserve `bodyPath` and `targetField` exactly as the current IR contract defines them.

- [ ] **Step 4: Run frontend and backend tests**

```bash
cd hydragrow-frontend
npm run test -- src/components/automation/reactflow/NodeEditorPanel.test.tsx src/components/automation/reactflow/WebhookFieldMappingEditor.test.tsx
cargo test --manifest-path hydragrow-backend/Cargo.toml webhook
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.tsx hydragrow-frontend/src/components/automation/reactflow/WebhookFieldMappingEditor.tsx hydragrow-frontend/src/lib/automation/ir.ts hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.test.tsx hydragrow-frontend/src/components/automation/reactflow/WebhookFieldMappingEditor.test.tsx
git commit -m "feat(automation): polish webhook trigger binding UI"
```

---

## Task 8: Make Chain action first-class and visually match reference `07`

**Files:**
- Modify: `hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.tsx`
- Modify: `hydragrow-frontend/src/components/automation/FlowEditorFooter.tsx`
- Modify: `hydragrow-frontend/src/components/automation/NextFlowSelector.tsx`
- Modify: `hydragrow-frontend/src/lib/automation/flowCycle.ts` only if tests expose current graph semantics bug
- Test: `hydragrow-frontend/src/components/automation/FlowDetailDrawer.test.tsx`
- Test: `hydragrow-frontend/src/lib/automation/flowCycle.test.ts`

- [ ] **Step 1: Add failing chain UX tests**

```tsx
it('groups candidate flows by kind and disables cycle-forming candidates', () => {
  render(<NextFlowSelector scripts={[alertFlow, actionFlow, recipeFlow, cyclicFlow]} selectedIds={[]} isCycle={(id) => id === cyclicFlow.id} onToggle={vi.fn()} />);
  expect(screen.getByText('Auto dose PH_DOWN')).toBeInTheDocument();
  expect(screen.getByText('Bổ sung dinh dưỡng')).toBeInTheDocument();
  expect(screen.getByText('Kết thúc mùa vụ')).toBeInTheDocument();
  expect(screen.getByText('không cho phép — sẽ tạo vòng lặp')).toBeInTheDocument();
});
```

- [ ] **Step 2: Render the chain preview from the reference**

Show `pH quá cao (alert) → Auto dose PH_DOWN (action)` style preview using real selected script names and kind badges. Keep cycle detection warning text visible and actionable.

- [ ] **Step 3: Add chain action node UI**

In the action editor, make `Chain → chạy Flow khác` visually distinct as an action capable of dispatching another Flow. Do not alter backend dispatch semantics already covered by `next_flow_ids`; the persisted source of truth remains `ir_json.next_flow_ids`.

- [ ] **Step 4: Verify cycle handling**

Run:
```bash
cd hydragrow-frontend
npm run test -- src/components/automation/FlowDetailDrawer.test.tsx src/lib/automation/flowCycle.test.ts
```
Expected: PASS, including cross-kind chain candidates and self/indirect-cycle rejection.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.tsx hydragrow-frontend/src/components/automation/FlowEditorFooter.tsx hydragrow-frontend/src/components/automation/NextFlowSelector.tsx hydragrow-frontend/src/lib/automation/flowCycle.ts hydragrow-frontend/src/components/automation/FlowDetailDrawer.test.tsx hydragrow-frontend/src/lib/automation/flowCycle.test.ts
git commit -m "feat(automation): make chain actions first-class in editor UI"
```

---

## Task 9: Rebuild dry-run Test Panel to match reference `08`

**Files:**
- Modify: `hydragrow-frontend/src/components/automation/reactflow/TestPanel.tsx`
- Modify: `hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx` only to integrate the new panel shell if required
- Modify: `hydragrow-frontend/src/types/automation.ts` only if current response types cannot render reference trace data
- Test: `hydragrow-frontend/src/components/automation/reactflow/TestPanel.test.tsx`
- Backend verification: current `/devices/{device_id}/scripts/test` endpoint and associated tests

- [ ] **Step 1: Write failing UI tests**

```tsx
it('renders sample inputs, run control, outcome banner, trace and actions', () => {
  render(<TestPanel deviceId="dev-1" ir={sampleIr} fields={['ph', 'ec', 'temp', 'water_level']} />);
  expect(screen.getByText('Giá trị mẫu để test')).toBeInTheDocument();
  expect(screen.getByText('Kết quả mô phỏng')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: /Chạy thử/i })).toBeInTheDocument();
  expect(screen.getByText('HÀNH ĐỘNG SẼ CHẠY')).toBeInTheDocument();
});
```

- [ ] **Step 2: Replace JSON-heavy output with structured trace cards**

Match `08`:
- left/upper sample input form with exact field names
- blue informational time-window note
- right/secondary result area
- red outcome banner when the flow fires, amber/neutral when it does not
- per-step trace rows with pass/fail icon, expression, actual value
- explicit action list such as `alert(...)` and `chain → ...`

Keep `actions_preview` as the backend source; only change presentation.

- [ ] **Step 3: Add deterministic sample defaults**

Seed the sample form only for first render of a test session, using sensible values that mirror the supplied reference (`ph=7.8`, `ec=2.1`, `temp=27.4`, `water_level=68`). Do not write those values into saved Flow IR.

- [ ] **Step 4: Run tests**

```bash
cd hydragrow-frontend
npm run test -- src/components/automation/reactflow/TestPanel.test.tsx
npm run build
```
Expected: PASS.

- [ ] **Step 5: Verify backend dry-run contract**

Run the repository's script-test endpoint tests and ensure `will_fire`, `trace`, and `actions_preview` remain unchanged.

- [ ] **Step 6: Commit**

```bash
git add hydragrow-frontend/src/components/automation/reactflow/TestPanel.tsx hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx hydragrow-frontend/src/types/automation.ts hydragrow-frontend/src/components/automation/reactflow/TestPanel.test.tsx
git commit -m "feat(automation): redesign dry-run test panel"
```

---

## Task 10: Rebuild multi-device template UI to match reference `09`

**Files:**
- Modify: `hydragrow-frontend/src/components/automation/MultiDeviceApplyDialog.tsx`
- Modify: `hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx` only for open/close plumbing if required
- Modify: relevant automation query hook file containing template operations (confirm exact path from current branch; history currently shows `MultiDeviceApplyDialog` already exists)
- Test: `hydragrow-frontend/src/components/automation/MultiDeviceApplyDialog.test.tsx`
- Backend verification: existing `apply-template` and `sync-template` routes/migrations

- [ ] **Step 1: Write failing tests for override semantics**

```tsx
it('shows per-device override state and keeps original overrides on sync', () => {
  render(<MultiDeviceApplyDialog sourceFlow={templateFlow} devices={[
    { id: 'a', name: 'Trạm A — Nhà kính 1', override: true, threshold: 'ph > 7.5' },
    { id: 'b', name: 'Trạm B — Nhà kính 2', override: false, threshold: 'ph > 7.5' },
    { id: 'c', name: 'Trạm C — Sân thượng', override: false, threshold: 'ph > 7.5' },
  ]} />);
  expect(screen.getByText('override')).toBeInTheDocument();
  expect(screen.getByText('gốc gốc')).toBeInTheDocument();
  expect(screen.getByText('Đồng bộ thay đổi')).toBeInTheDocument();
});
```

- [ ] **Step 2: Implement reference `09` layout**

Show selected devices as full-width rows with:
- checkbox
- device name
- override/original badge
- threshold/summary badge
- selected count + primary apply button
- muted sync explanation

Preserve the current rule from the reference: sync updates only devices that have not overridden the template.

- [ ] **Step 3: Keep backend payloads unchanged**

Use existing `apply-template` and `sync-template` mutations. No new persistence shape unless the existing API cannot represent the reference behavior.

- [ ] **Step 4: Run frontend and backend tests**

```bash
cd hydragrow-frontend
npm run test -- src/components/automation/MultiDeviceApplyDialog.test.tsx
cargo test --manifest-path hydragrow-backend/Cargo.toml template
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-frontend/src/components/automation/MultiDeviceApplyDialog.tsx hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx hydragrow-frontend/src/components/automation/MultiDeviceApplyDialog.test.tsx
git commit -m "feat(automation): redesign multi-device template dialog"
```

---

## Task 11: Remove remaining automation UI debt and enforce visual consistency

**Files:**
- Modify: `hydragrow-frontend/src/pages/Automation.tsx`
- Modify: `hydragrow-frontend/src/components/automation/**/*.tsx` files changed by Tasks 1–10
- Modify: `hydragrow-frontend/src/App.css`
- Modify/remove: obsolete Blockly automation components only after repository-wide usage search proves React Flow is the sole editor
- Test: `hydragrow-frontend/src/pages/Automation.test.tsx`

- [ ] **Step 1: Search for forbidden/stale styles and old editor references**

Run:
```bash
cd hydragrow-frontend
grep -RInE "bg-blue-|text-blue-|border-blue-|text-gray-|bg-gray-|slate-|Blockly|blockly/extractIr" src/components/automation src/pages/Automation.tsx
```
Expected: no new violations in the final Automation UI; any remaining legacy reference must be explained by a still-used non-automation feature before removal.

- [ ] **Step 2: Search for duplicate automation flows**

Confirm only React Flow remains in the Automation route. The UI standard explicitly calls for one desktop editor, not parallel Blockly + React Flow editors.

- [ ] **Step 3: Verify responsive states**

Test at:
- 390×844 mobile: flow list/cards, no draggable canvas
- 768×1024 tablet: flow list/cards, editor in stacked panel
- 1440×900 desktop: overview canvas + drawer/editor
- narrow desktop/Tauri: no overflow caused by fixed drawer widths

- [ ] **Step 4: Run full frontend gates**

```bash
cd hydragrow-frontend
npm run test
npm run lint
npm run build
```
Expected: PASS.

- [ ] **Step 5: Run automation backend regression suite**

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add hydragrow-frontend/src/pages/Automation.tsx hydragrow-frontend/src/components/automation hydragrow-frontend/src/App.css
 git commit -m "refactor(automation): remove UI debt and enforce design tokens"
```

---

## Acceptance checklist

- [ ] Reference `00`: Automation UI uses only the documented token system; no new blue/slate/gray automation styling.
- [ ] Reference `01`: overview canvas shows flow summary cards, trigger badges, enabled/disabled state, and animated chain edges.
- [ ] Reference `02`: palette exposes Trigger, Condition, Delay, Action groups with all phase capabilities visible.
- [ ] Reference `03`: nested AND/OR editor works, including expression preview and child groups.
- [ ] Reference `04`: mean/min/max condition UI shows window in minutes and survives IR/compiler round-trip.
- [ ] Reference `05`: Cron tab is real UI, not placeholder; next-run preview works.
- [ ] Reference `06`: Webhook tab shows URL, mode, field mappings, and flow usage summary.
- [ ] Reference `07`: Chain action can select cross-kind target Flows while cycle-forming candidates are disabled.
- [ ] Reference `08`: dry-run shows sample values, outcome banner, condition trace, and concrete actions.
- [ ] Reference `09`: multi-device template dialog shows selection, override status, threshold summary, apply and sync behavior.
- [ ] Mobile does not embed draggable React Flow canvas.
- [ ] Desktop/Tauri keeps one React Flow editor.
- [ ] Existing backend contracts remain compatible; no wire-format changes from a UI-only redesign.

---

## Parallel Execution Lanes

Apply the parallel-worktree rule from `parallel-worktree-sessions`: partition by actual file overlap, not feature labels. Tasks sharing a file stay in the same lane; shared foundations merge first. One branch + one worktree + one session per lane; merge lanes serially from the integration worktree.

### Lane 0 — foundation (must finish first)
- Branch: `lane/automation-foundation`
- Tasks: 1
- Ownership: shared drawer chrome, shared page header primitives, `App.css`, initial `FlowDetailDrawer` split

Create from the post-foundation base:
```bash
git worktree add ../HYDRAGROW-lane-automation-foundation -b lane/automation-foundation main
```

### Lane 1 — overview
- Branch: `lane/automation-overview`
- Tasks: 2
- Files owned: `Automation.tsx`, `useFlowCanvas.ts`, `FlowSummaryNode.tsx`, overview tests

Create only after Lane 0 merges:
```bash
git worktree add ../HYDRAGROW-lane-automation-overview -b lane/automation-overview main
```

### Lane 2 — editor core
- Branch: `lane/automation-editor-core`
- Tasks: 3, 4, 5
- Files owned: `NodePalette.tsx`, `useAutomationBuilder.ts`, `ConditionGroupEditor.tsx`, `conditionTree.ts`, related tests, editor panel styling

Because these tasks intentionally touch the same editor surface, execute sequentially inside this lane.

Create after Lane 0 merges:
```bash
git worktree add ../HYDRAGROW-lane-automation-editor-core -b lane/automation-editor-core main
```

### Lane 3 — trigger integrations
- Branch: `lane/automation-triggers`
- Tasks: 6, 7
- Files owned: trigger editor portions of `NodeEditorPanel.tsx`, `WebhookFieldMappingEditor.tsx`, Cron hook/tests, trigger-focused tests

This lane starts only after Lane 0 merges and Lane 2 has established the editor-core component contract. Do not edit files owned by Lane 2 outside the agreed trigger sections.

Create:
```bash
git worktree add ../HYDRAGROW-lane-automation-triggers -b lane/automation-triggers main
```

### Lane 4 — chain + dry-run + templates
- Branch: `lane/automation-advanced`
- Tasks: 8, 9, 10
- Files owned: `NextFlowSelector.tsx`, `TestPanel.tsx`, `MultiDeviceApplyDialog.tsx`, advanced-flow tests

Execute sequentially because `FlowDetailDrawer` and shared advanced-flow controls can otherwise overlap.

Create:
```bash
git worktree add ../HYDRAGROW-lane-automation-advanced -b lane/automation-advanced main
```

### Lane 5 — final integration cleanup
- Branch: `lane/automation-cleanup`
- Tasks: 11
- Runs only after Lanes 1–4 merge serially.

Create:
```bash
git worktree add ../HYDRAGROW-lane-automation-cleanup -b lane/automation-cleanup main
```

### Serial merge order

```bash
git merge lane/automation-foundation
# run frontend tests

git merge lane/automation-overview
# run frontend tests

git merge lane/automation-editor-core
# run frontend tests

git merge lane/automation-triggers
# run frontend + automation backend tests

git merge lane/automation-advanced
# run frontend + automation backend tests

git merge lane/automation-cleanup
# run full gates
```

After each merged lane, remove its worktree and delete the merged branch:
```bash
git worktree remove ../HYDRAGROW-lane-automation-foundation
git branch -d lane/automation-foundation
```

Do the equivalent for every lane. Do not merge lanes concurrently.

---

## Self-review

**Spec coverage:** Tasks 1–11 map to every supplied reference frame `00`–`09`, plus responsive/mobile requirements already established by the repository UI standard.

**Placeholder scan:** No `TBD`, `TODO`, `implement later`, or generic “write tests” steps. Every task names exact files and explicit commands.

**Type consistency:** Existing types remain authoritative: `AutomationIr`, `ConditionOrGroup`, `Action`, `WebhookTriggerConfig`, `next_flow_ids`. UI-only extraction does not create a second IR.

**Known repository drift to verify before coding:** the current fetched `useFlowCanvas.ts` still returns `edges: []` and contains an old “no edge / Blockly” comment even though repository history contains an AUTOMATION-003 chain-edge commit; current `NodeEditorPanel.tsx` search results still show a Cron placeholder although AUTOMATION-005 history says Cron UI exists. Implementation must inspect the exact branch contents and resolve this drift explicitly rather than layering a second implementation on top.
