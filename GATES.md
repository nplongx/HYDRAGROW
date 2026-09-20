# Gates: Dashboard P3 spec reconciliation

OWNS: hydragrow-frontend/src/pages/Dashboard.tsx, hydragrow-frontend/src/pages/Dashboard.test.tsx, hydragrow-frontend/src/**, harness/build-log.md

Scope: Reconcile Dashboard with the supplied functional specification without inventing unsupported data, duplicating global state, or expanding into Fleet/Operations responsibilities.

- [ ] G0: gate ledger is structurally valid and contains actionable oracles
  CHECK: node .agents/skills/unlazy/scripts/gate-lint.mjs GATES.md
  EXPECT: LINT OK
  EVIDENCE: pending

- [ ] G1: All Stations Overview is the Dashboard default monitoring surface when multiple accessible stations exist
  CHECK: node .unlazy/dashboard/verify-dashboard-gates.mjs overview-default
  EXPECT: DASHBOARD_OVERVIEW_DEFAULT_PASS
  EVIDENCE: pending

- [ ] G2: All Stations Overview exposes only source-backed fleet summary and monitoring controls required by the spec
  CHECK: node .unlazy/dashboard/verify-dashboard-gates.mjs overview-contract
  EXPECT: DASHBOARD_OVERVIEW_CONTRACT_PASS
  EVIDENCE: pending

- [ ] G3: Station cards provide source-backed identity, connection, attention, EC, pH, crop, and warning count without fabricated metrics
  CHECK: node .unlazy/dashboard/verify-dashboard-gates.mjs station-card-contract
  EXPECT: DASHBOARD_STATION_CARD_PASS
  EVIDENCE: pending

- [ ] G4: Selecting a station transfers authoritative station context into Selected Station Detail and keeps identity explicit
  CHECK: node .unlazy/dashboard/verify-dashboard-gates.mjs station-selection
  EXPECT: DASHBOARD_STATION_SELECTION_PASS
  EVIDENCE: pending

- [ ] G5: Selected Station Detail preserves the specified source-backed monitoring regions and does not become a duplicate Operations/Cultivation/Fleet surface
  CHECK: node .unlazy/dashboard/verify-dashboard-gates.mjs selected-detail-contract
  EXPECT: DASHBOARD_SELECTED_DETAIL_PASS
  EVIDENCE: pending

- [ ] G6: Loading, empty, filtered-empty, fleet retrieval error, selected-station loading, and offline states follow the supplied Dashboard contracts
  CHECK: node .unlazy/dashboard/verify-dashboard-gates.mjs state-contract
  EXPECT: DASHBOARD_STATE_CONTRACT_PASS
  EVIDENCE: pending

- [ ] G7: Dashboard station switching and cross-page handoff preserve identifiable station context using existing architecture
  CHECK: node .unlazy/dashboard/verify-dashboard-gates.mjs context-persistence
  EXPECT: DASHBOARD_CONTEXT_PERSISTENCE_PASS
  EVIDENCE: pending

- [ ] G8: Safety interaction semantics do not claim physical command success from a UI click, and E-STOP behavior matches the canonical immediate-action contract
  CHECK: node .unlazy/dashboard/verify-dashboard-gates.mjs safety-contract
  EXPECT: DASHBOARD_SAFETY_CONTRACT_PASS
  EVIDENCE: pending

- [ ] G9: Dashboard tests cover the overview/detail/context contracts and all relevant states
  CHECK: node .unlazy/dashboard/verify-dashboard-gates.mjs tests
  EXPECT: DASHBOARD_TESTS_PASS
  EVIDENCE: pending

- [ ] G10: production build succeeds after Dashboard reconciliation
  CHECK: npm run build:web
  EXPECT: built in
  CWD: hydragrow-frontend
  EVIDENCE: pending

- [ ] G11: lint has zero errors after Dashboard reconciliation
  CHECK: npm run lint
  EXPECT: 0 errors
  CWD: hydragrow-frontend
  EVIDENCE: pending

- [ ] G12: repository diff is whitespace-clean
  CHECK: git diff --check
  EXPECT: DASHBOARD_DIFF_CHECK_PASS
  EVIDENCE: pending

- [ ] G13: authenticated browser review confirms the implemented Dashboard matches the overview/detail interaction contract
  EVIDENCE: pending

- [ ] G14: final evidence and build log state Dashboard acceptance truthfully, with no unsupported acceptance claim
  CHECK: node .unlazy/dashboard/verify-dashboard-gates.mjs evidence
  EXPECT: DASHBOARD_EVIDENCE_PASS
  EVIDENCE: pending
