# HYDRAGROW User Discovery & Strategic Research Report

**Document ID:** `DISC-2026-09-12-001`  
**Date:** 2026-09-12  
**Revision:** Revision 3: added item 11 Competitive Analysis.  
**Status:** PROPOSED / DISCOVERY  
**Scope:** Design Research, UX Strategy, User Personas, Journey Mapping, Technical Pre-conditions  
**Governance Reference:** `docs/DELIVERY-GOVERNANCE.md`  
**Inputs:** `README.md`, `AGENTS.md`, `docs/hydragrow-hifi-spec.md`, `hydragrow-backend/migrations/`, `hydragrow-backend/src/`, `hydragrow-frontend/src/`

> **Changelog / Revision Note:**  
> Revision 2: Item 8 re-labeled as 'assumption'—see review history for the reason. Section 8 constants (Z-score bounds, time horizon, dosing percentages, growth delay thresholds) are explicitly designated as uncalibrated assumptions pending live InfluxDB telemetry. An actionable calibration procedure (§8.3) was added for execution upon gaining database access. Section 9 table was updated to denote immediate actionability for the two verified authorization bugs in `api/alert.rs` and `api/recipe.rs`.  
> Revision 3: added item 11 Competitive Analysis. Section 11 benchmarks the three JTBD tracks defined in §4 against publicly documented consumer hydroponic products (Track A), commercial greenhouse control systems (Track B), and agronomic R&D tooling (Track C), identifies capability gaps for each track relative to HYDRAGROW, and revisits open question (1) in §9 in light of whether the hobbyist and commercial competitive landscapes are served by separate products or a single tiered platform. Sections 1–10 are unmodified; this is an append-only addition.

---

## 1. Executive Summary

This user discovery study clarifies **who** is (and will be) using the HYDRAGROW smart aeroponic/hydroponic system and **why**, establishing an empirical baseline before committing to UI redesigns, feature expansions, or architecture reorganizations.

Prior to this study, the project operated under a high-fidelity specification (`docs/hydragrow-hifi-spec.md`) that identified three unresolved architectural questions (line 118):
1. Is the product priority a single-station household setup or a multi-station commercial setup?
2. What are the strict permission boundaries between "Operator" and "Viewer"?
3. Are FCM/APNs notification pipelines ready for production deployment?

Furthermore, existing alert and deviation thresholds in the specification (e.g., "abnormal nutrient dosing >X% over Y hours", "3-day delay relative to formula", "repeat offline 5 times in 10 minutes") were hard-coded estimates lacking data-driven mathematical models.

By examining the database schemas, backend and frontend codebases, API scope enforcement, and telemetry schemas, this document separates **empirically supported facts** from **unverified design assumptions**, defines Jobs-To-Be-Done (JTBD) and Persona hypotheses, audits the Information Architecture (IA) against a complete crop lifecycle journey, provides an interview guide to resolve open questions with real users, and outlines a quantitative sensor analytics framework using InfluxDB time-series data.

---

## 2. Design Brief

### 2.1 Project Overview
- **Product Name:** HYDRAGROW
- **Mission:** Autonomous precision monitoring and closed-loop dosing control for automated aeroponic and hydroponic cultivation systems (`README.md`, lines 3–13).
- **Subsystem Ecosystem:**
  - `ESP32-C3-SENSOR-NODE`: C++/PlatformIO telemetry node measuring Electrical Conductivity (EC), pH, water temperature, and ultrasonic water level (`README.md`, lines 8–10).
  - `ESP32-C3-CONTROLLER-NODE`: Embedded Rust firmware running a Finite State Machine (FSM) controlling peristaltic dosing pumps (Nutrient A, Nutrient B, pH Up, pH Down), misting valves, and circulation pumps with MIMO adaptive/Kalman filtering (`README.md`, lines 9–11).
  - `hydragrow-backend`: Rust / Actix-web server handling MQTT telemetry ingestion, InfluxDB time-series storage, PostgreSQL relational state, WebSocket fan-out, and user authentication via Firebase Auth (`README.md`, lines 16–23).
  - `hydragrow-frontend`: React / TypeScript / Tauri desktop and web application with Zustand state management (`README.md`, lines 28–32).

### 2.2 Problem Statement
- **What:** Aeroponic and hydroponic crops (e.g., leafy greens, herbs, fruiting vegetables) depend entirely on nutrient solutions directly delivered to plant roots. Root death or irreversible crop damage occurs within 2–6 hours if misting fails, if water depletes, or if EC/pH values drift into toxic or lockout ranges (`docs/hydragrow-hifi-spec.md`, lines 60, 115).
- **Who:** Crop caretakers ranging from home balcony gardeners to commercial farm supervisors who cannot maintain continuous 24/7 physical presence at the reservoir tank.
- **Evidence of Current Pain:**
  - Manual chemical measurement and syringe titration are labor-intensive and prone to human error (e.g., simultaneous acid and base addition, causing hazardous neutralization reactions, addressed in `hydragrow-backend/src/api/control.rs`, lines 400–410).
  - Lack of timely push alerts leads to silent tank starvation or pump stalls before physical inspection discovers root wilt.
  - Multi-station operators lack unified cross-station visibility in mobile viewports (`docs/hydragrow-hifi-spec.md`, lines 109–113).
- **Consequences of Failure:** Total crop mortality, wasted electricity and nutrients, loss of commercial harvest yield, and user abandonment of automated hydroponics.

### 2.3 Evidence-Based User Groups vs. Assumption-Based User Groups

To adhere to the principle of "no guesswork," all user classifications are divided into those explicitly defined in the source code/schema and those currently existing as product hypotheses.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           EVIDENCE HIERARCHY                            │
├────────────────────────────────────┬────────────────────────────────────┤
│ EVIDENCE-SUPPORTED GROUPS           │ ASSUMPTION-BASED GROUPS            │
│ (Explicit in Schema & API Scopes)   │ (Hypothesized Product Personas)    │
├────────────────────────────────────┼────────────────────────────────────┤
│ 1. System Admin / Deployer         │ A. Urban Balcony Hobbyist          │
│    (scopes: '*', 'device:admin')   │    - 1 station, set-and-forget     │
│ 2. Farm Station Operator           │ B. Commercial Greenhouse Manager   │
│    (scopes: 'control:*', 'write:*')│    - Multi-station, staff shifts   │
│ 3. Read-Only Viewer / Stakeholder  │ C. Agronomist / R&D Specialist     │
│    (scope: 'read:telemetry')       │    - Recipe prototyping, analytics │
└────────────────────────────────────┴────────────────────────────────────┘
```

#### Evidence-Supported User Groups (Code & Schema Provenance)
1. **System Administrator / Deployer (`role = 'admin'`):**
   - *Provenance:* Defined in `hydragrow-backend/migrations/20260910100004_add_role_to_users.sql` (lines 14–15), `hydragrow-backend/src/api/scope_definitions.rs` (lines 18, 25, 42), and `hydragrow-frontend/src/pages/Roles.tsx` (lines 21, 68–73).
   - *Capabilities:* Possesses scopes `*` and `device:admin`. Responsible for provisioning users, binding hardware serials (`hydragrow-backend/migrations/20260907000001_device_hardware_binding.sql`), initiating OTA firmware updates (`device:ota`), resetting factory defaults, and configuring device WiFi fallback networks (`device:network`).
2. **Station Operator (`role = 'operator'`):**
   - *Provenance:* Defined in `hydragrow-backend/migrations/20260910100004_add_role_to_users.sql` (lines 16–17) and `hydragrow-frontend/src/pages/Roles.tsx` (lines 22–31).
   - *Capabilities:* Possesses scopes `read:telemetry`, `write:config`, `control:pump`, `control:emergency`, `device:ota`, `device:network`, `script:write`, `recipe:write`. Can start/finish crop seasons, create and apply recipes, trigger manual pump dosing, adjust PWM rates, trigger E-stop, and author automation logic chains.
3. **Telemetry Viewer (`role = 'viewer'`):**
   - *Provenance:* Defined in `hydragrow-backend/migrations/20260910100004_add_role_to_users.sql` (line 18) and `hydragrow-frontend/src/pages/Roles.tsx` (line 32).
   - *Capabilities:* Possesses only `read:telemetry`. Designed for read-only monitoring of live EC, pH, water level, temperature, and historical charts.

#### Assumption-Based User Groups (Unverified Product Hypotheses)
1. **Urban Balcony / Single-Station Home Grower:**
   - *Hypothesis:* A consumer who owns exactly one aeroponic tower on an apartment balcony. Assumed to want a high degree of automation ("set-and-forget"), visual plant tracking (photo journals), and simple push notifications without technical complexity.
   - *Status:* `ASSUMPTION — requires verification`. No demographic studies, customer logs, or feedback records exist in the repository.
2. **Commercial Multi-Station Farm Supervisor:**
   - *Hypothesis:* A professional manager overseeing 5 to 50+ growing racks in a greenhouse or warehouse. Assumed to require batch recipe application (`hydragrow-backend/src/services/multi_device_template.rs`), multi-station health summaries (`FleetView.tsx`), technician shift management, and exportable compliance logs.
   - *Status:* `ASSUMPTION — requires verification`. While backend multi-tenant tables (`device_ownership`) and fleet endpoints (`/api/fleet/summary`) exist, no real commercial customer deployments are documented.
3. **Agronomist / Recipe Developer:**
   - *Hypothesis:* A hydroponics specialist designing nutrient formulations across crop stages (seedling, vegetative, flowering, harvest) who relies on granular InfluxDB telemetry and Grafana analytics.
   - *Status:* `ASSUMPTION — requires verification`.

### 2.4 Codebase Clues & Seed Data Audit
- **Database Seed Data:**
  - PostgreSQL migrations contain **no production user seeds** or sample customer deployments (`hydragrow-backend/migrations/`).
  - Rust database unit tests (`hydragrow-backend/src/db/tests/test_users.rs`, lines 10, 26, 55; `test_fcm_tokens.rs`, line 7) insert synthetic identifiers such as `"firebase-uid-abc"`, `"user1@example.com"`, and `"device_001"`.
  - Frontend development relies on a client-side mock authentication toggle (`hydragrow-frontend/src/App.tsx`, lines 43–51; `platform/http.ts`, lines 44–46) that hardcodes `mock_user_id = '1'`.
- **Ownership Model Schema Implication:**
  - In `hydragrow-backend/migrations/20260823100000_device_ownership.sql` (lines 2–9):
    ```sql
    CREATE TABLE device_ownership (
        id         BIGSERIAL PRIMARY KEY,
        user_id    BIGINT       NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        device_id  TEXT         NOT NULL,
        label      TEXT,
        claimed_at TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UNIQUE (user_id, device_id)
    );
    ```
  - *Architectural Finding:* The constraint `UNIQUE (user_id, device_id)` enforces uniqueness **per user-device pair**, NOT per device. This architecture natively allows:
    1. A single user to own multiple devices ($1 : N$).
    2. Multiple users to claim the same device ($N : M$), enabling team/shared station management.
  - *Frontend Architectural Dissonance:* In `hydragrow-frontend/src/store/useDeviceStore.ts` and `useDeviceSync.ts`, the frontend state assumes a **single active `deviceId`** at any given time. `FleetView.tsx` is an isolated route (`/fleet`), while the main 5-tab navigation (`MainLayout.tsx`, lines 34–40) is strictly scoped to a single station.

---

## 3. Database Telemetry & User Distribution Query Methodology

To answer Open Question (1) ("Is the priority a single-station household or a multi-station commercial setup?"), empirical data must be extracted from the production database. The following SQL queries are designed to run directly against PostgreSQL to determine real user-to-device topology and operational cadence.

### 3.1 SQL Query Specification

#### Query 1: Overall User-to-Device Ratio & Fleet Topology
```sql
-- Computes aggregate system scale and average stations per user
SELECT 
    COUNT(DISTINCT u.id) AS total_registered_users,
    COUNT(DISTINCT do.device_id) AS total_claimed_devices,
    COUNT(do.id) AS total_ownership_links,
    ROUND(COUNT(do.id)::numeric / NULLIF(COUNT(DISTINCT u.id), 0), 2) AS avg_devices_per_user,
    ROUND(COUNT(do.id)::numeric / NULLIF(COUNT(DISTINCT do.device_id), 0), 2) AS avg_users_per_device
FROM users u
LEFT JOIN device_ownership do ON u.id = do.user_id;
```

#### Query 2: Single-Station vs. Multi-Station User Segmentation
```sql
-- Categorizes each user by device count to reveal user segment distribution
WITH user_device_counts AS (
    SELECT 
        u.id AS user_id,
        u.email,
        u.role,
        COUNT(do.device_id) AS device_count
    FROM users u
    LEFT JOIN device_ownership do ON u.id = do.user_id
    GROUP BY u.id, u.email, u.role
)
SELECT 
    CASE 
        WHEN device_count = 0 THEN '0 devices (Registered / Inactive)'
        WHEN device_count = 1 THEN '1 device (Single-Station Household / Micro)'
        WHEN device_count BETWEEN 2 AND 4 THEN '2-4 devices (Multi-Station Enthusiast / Semi-Commercial)'
        ELSE '5+ devices (Commercial Fleet Operator)'
    END AS user_segment,
    COUNT(*) AS user_count,
    ROUND(COUNT(*)::numeric / SUM(COUNT(*)) OVER() * 100, 1) AS pct_of_total_users
FROM user_device_counts
GROUP BY 1
ORDER BY MIN(device_count);
```

#### Query 3: Multi-User Collaboration on Shared Devices
```sql
-- Determines whether devices are operated by solitary individuals or collaborative teams
WITH device_collaborators AS (
    SELECT 
        device_id,
        COUNT(user_id) AS active_operators
    FROM device_ownership
    GROUP BY device_id
)
SELECT 
    CASE 
        WHEN active_operators = 1 THEN 'Single-user Station'
        ELSE 'Shared / Multi-operator Station'
    END AS collaboration_model,
    COUNT(*) AS device_count,
    ROUND(COUNT(*)::numeric / SUM(COUNT(*)) OVER() * 100, 1) AS pct_of_total_devices
FROM device_collaborators
GROUP BY 1;
```

#### Query 4: Crop Season Creation Frequency & Active Utilization Cadence
```sql
-- Measures how actively users start, maintain, and finish crop seasons
SELECT 
    DATE_TRUNC('month', start_time) AS season_cohort_month,
    COUNT(*) AS total_seasons_started,
    COUNT(DISTINCT device_id) AS active_stations,
    ROUND(COUNT(*)::numeric / NULLIF(COUNT(DISTINCT device_id), 0), 2) AS seasons_per_station,
    COUNT(CASE WHEN status = 'active' THEN 1 END) AS currently_active,
    COUNT(CASE WHEN status = 'completed' THEN 1 END) AS successfully_completed,
    AVG(CASE 
        WHEN end_time IS NOT NULL THEN EXTRACT(DAY FROM (end_time - start_time))
    END)::numeric(10, 1) AS avg_completed_crop_days
FROM crop_seasons
GROUP BY DATE_TRUNC('month', start_time)
ORDER BY season_cohort_month DESC;
```

### 3.2 Execution Status & Finding on Open Question (1)
- **Execution Check:** In this offline development sandbox, the PostgreSQL server is running locally on port 5432, but peer authentication for the default user is restricted, and production environment credentials (`hydragrow-backend/.env`) are untracked per repository security policies (`.gitignore`, line 13).
- **Authoritative Determination on Open Question (1):**
  > **INSUFFICIENT DATA — requires executing the above queries on production database and conducting user interviews.**
  
  The system architecture permits both paradigms, but empirical deployment proportions remain unmeasured. We must not arbitrarily default to single-station or commercial fleet without production query evidence.

---

## 4. Jobs-To-Be-Done (JTBD) Hypotheses

Applying the Clayton Christensen / Tony Ulwick JTBD framework (`.agent/designer-skills/design-research/skills/jobs-to-be-done/SKILL.md`), we formulate three competing job hypotheses that explain why users would hire HYDRAGROW.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            JTBD HYPOTHESIS SPECTRUM                         │
├──────────────────────────┬──────────────────────────┬───────────────────────┤
│ Track A: Peace of Mind   │ Track B: Commercial Ops  │ Track C: Agronomic    │
│ & Passive Care           │ & Yield Optimization     │ Research & R&D        │
├──────────────────────────┼──────────────────────────┼───────────────────────┤
│ "Keep my home plants     │ "Maximize harvest volume │ "Formulate, test, and │
│ alive autonomously while │ across multiple benches  │ refine plant recipes  │
│ I work and travel."      │ with zero operator error"│ with precision data." │
└──────────────────────────┴──────────────────────────┴───────────────────────┘
```

### 4.1 JTBD Track A: "Autonomous Care & Peace of Mind via Remote Guardrails"
- **Job Statement:**
  > *When* I am busy with work, family, or traveling away from home,  
  > *I want to* trust the automated system to maintain nutrients and water levels within safe biological bounds,  
  > *So I can* enjoy fresh, clean greens without feeling tethered to daily manual testing or worrying about crop death.
- **Job Dimensions:**
  - **Functional:** Keep pH (5.8–6.5) and EC (1.2–1.8 mS/cm) stable; automatically mist roots every 15 minutes; shut off pumps if the tank runs dry; notify immediately if an urgent intervention is needed.
  - **Emotional:** Freedom from anxiety over plant neglect; relief that an expensive aeroponic rig will not fail silently; pride in seeing plants thrive.
  - **Social:** Pride in sharing home-grown, pesticide-free vegetables with family and dinner guests; demonstrating a modern, high-tech sustainable lifestyle.
- **Job Lifecycle Stages:**
  1. *Define:* Choose a simple crop (e.g., butterhead lettuce).
  2. *Locate:* Buy seedling plugs and liquid nutrients (Part A & B).
  3. *Prepare:* Fill reservoir with tap/RO water; calibrate pH probe.
  4. *Confirm:* Verify sensor readings match expected water baseline.
  5. *Execute:* Start crop season in app; set to automatic control mode.
  6. *Monitor:* Glance at phone dashboard once a day during morning commute.
  7. *Modify:* Add water or top up nutrient bottles only when notified.
  8. *Conclude:* Harvest outer leaves; log final photo; clean basin.
- **Current Alternatives:** Manual dip-test pens, analog lamp timers, gravity drip systems, asking a neighbor to water plants while traveling.
- **Underserved Needs:** Manual testing is messy and easily neglected; existing hobby systems lack reliable automatic pH/EC correction and push notifications.

### 4.2 JTBD Track B: "Standardized Productivity & Yield Optimization at Scale"
- **Job Statement:**
  > *When* managing multiple commercial growing channels across a greenhouse facility,  
  > *I want to* deploy verified nutrient recipes in bulk, delegate routine operations to shift workers with strict safety bounds, and track consumption trends,  
  > *So I can* maximize sellable crop biomass per square meter, eliminate operator errors, and maintain predictable harvest schedules.
- **Job Dimensions:**
  - **Functional:** Apply recipe templates across 10+ stations simultaneously; lock out junior technicians from modifying hazardous thresholds; audit chemical consumption (ml of Nutrient A/B and acid per day); log all manual overrides.
  - **Emotional:** Confidence in commercial profitability; security against catastrophic staff error (e.g., dosing 500 ml of nitric acid due to mistyped decimal points).
  - **Social:** Professional credibility as a certified Good Agricultural Practices (GAP) supplier; meeting commercial supply contracts on schedule.
- **Job Lifecycle Stages:**
  1. *Define:* Plan commercial crop cycle targets (e.g., 2,000 heads of bok choy for wholesale delivery on Day 32).
  2. *Locate:* Allocate greenhouse bays HG-01 through HG-08.
  3. *Prepare:* Pump-calibrate dosing lines; test Wi-Fi and MQTT connectivity.
  4. *Confirm:* Bulk-apply standard recipe template across all assigned stations.
  5. *Execute:* Assign daily operational checklists to shift operators.
  6. *Monitor:* Review `/fleet/summary` status screen; inspect anomaly banners and 24h dosing distributions.
  7. *Modify:* Fine-tune EC targets during vegetative expansion; acknowledge alerts.
  8. *Conclude:* Record yield weight (kg); export CSV logs for traceability; initiate CIP (Clean-In-Place) sanitation.
- **Current Alternatives:** Industrial PLC cabinets (Priva, Argus), paper clipboards hung on tanks, manual dosing logs, standalone data loggers.
- **Underserved Needs:** Industrial systems are prohibitively expensive ($10k+); cheaper IoT tools lack granular role-based access control and multi-station template deployment.

### 4.3 JTBD Track C: "Empirical Agronomic Research & Recipe Optimization"
- **Job Statement:**
  > *When* testing new crop cultivars, light-to-nutrient interactions, or alternative nutrient salts,  
  > *I want to* inspect high-frequency sensor time-series, correlate chemical uptake with growth stages, and run automation scripts,  
  > *So I can* author, validate, and publish high-performance hydroponic growth recipes with statistical confidence.
- **Job Dimensions:**
  - **Functional:** Access raw InfluxDB sensor points; inspect MIMO/Kalman filter estimations; script conditional dosing flows with Rhai logic (`hydragrow-backend/src/services/script_engine.rs`).
  - **Emotional:** Curiosity; intellectual satisfaction from finding optimal biological growth curves.
  - **Social:** Academic publication, consulting authority, or proprietary competitive advantage.
- **Current Alternatives:** Custom Python scripts, Arduino dataloggers, laboratory spectrometer testing.
- **Underserved Needs:** Disconnected logging tools require hours of manual Excel correlation between sensor curves and dosing events.

---

## 5. User Personas (Hypotheses — Unverified)

In accordance with `.agent/designer-skills/design-research/skills/user-persona/SKILL.md`, the following two personas represent the archetypal users corresponding to JTBD Tracks A and B. Both are clearly labeled as hypotheses pending empirical validation.

---

### Persona 1: Minh — The Busy Urban Balcony Grower
*Status: `HYPOTHESIS — unverified`*

```
┌────────────────────────────────────────────────────────────────────────┐
│ MINH (34, Office Worker / Apartment Resident, HCMC)                    │
│ "Tôi muốn rau tươi sạch cho gia đình nhưng không thể túc trực          │
│  canh bồn nước mỗi ngày."                                              │
├────────────────────────────────────────────────────────────────────────┤
│ TECH COMFORT: High (Mobile App / Cloud Services)                       │
│ AGRONOMY KNOWLEDGE: Beginner to Intermediate (Reads online guides)     │
│ HARDWARE SETUP: 1 Aeroponic Tower (45 plant sites) on condo balcony    │
├────────────────────────────────────────────────────────────────────────┤
│ GOALS:                                                                 │
│ - Set-and-forget nutrient balance for leafy greens.                    │
│ - Instant phone notification if water runs out or EC/pH goes wild.     │
│ - Clear, jargon-free status indicators (Healthy vs. Action Needed).    │
│ - Visual photo journal to watch plants grow over 30 days.              │
├────────────────────────────────────────────────────────────────────────┤
│ FRUSTRATIONS & PAIN POINTS:                                            │
│ - Intimidated by technical jargon: "MIMO", "Kalman", "PWM 72%".        │
│ - Anxiety when app says "Mất kết nối Wi-Fi" without explaining if the  │
│   plants will still be watered while offline.                          │
│ - Messy pH probe calibration with buffer powder solutions.             │
├────────────────────────────────────────────────────────────────────────┤
│ SCENARIO (DAY IN THE LIFE):                                            │
│ Minh sips coffee before work and opens HYDRAGROW. The dashboard shows  │
│ a green "Hệ thống khỏe mạnh - Ngày 14/30" pill. Minh takes a photo     │
│ of his lettuce heads and heads to the office. At 14:30, his phone      │
│ buzzes: "Mực nước bồn thấp (<20%) - Hệ thống vẫn an toàn trong 12 giờ".│
│ He tops up the reservoir with 10 liters of tap water when he gets home.│
├────────────────────────────────────────────────────────────────────────┤
│ UX & DESIGN IMPLICATIONS:                                              │
│ - Prioritize single-station mobile viewport.                           │
│ - Hide complex automation scripting behind advanced settings.          │
│ - Explicit offline reassurance: "Offline nhưng bơm vẫn chạy lịch trình"│
│ - Direct photo capture button accessible from the dashboard.           │
└────────────────────────────────────────────────────────────────────────┘
```

---

### Persona 2: Anh Thắng — The Commercial Green-Farm Manager
*Status: `HYPOTHESIS — unverified`*

```
┌────────────────────────────────────────────────────────────────────────┐
│ ANH THẮNG (42, Greenhouse Operations Manager, Da Lat)                  │
│ "Một sai sót làm cháy rễ 10 giàn rau là mất cả trăm triệu.             │
│  Tôi cần kiểm soát đồng bộ và biết chính xác ai đã chỉnh gì."          │
├────────────────────────────────────────────────────────────────────────┤
│ TECH COMFORT: Moderate (Tablets, ERP, PLC, Excel)                      │
│ AGRONOMY KNOWLEDGE: Expert (Degree in Agronomy, 12 years farming)      │
│ HARDWARE SETUP: 16 NFT Troughs across 2 greenhouse bays, 4 staff       │
├────────────────────────────────────────────────────────────────────────┤
│ GOALS:                                                                 │
│ - Fleet Overview at a glance: identify which troughs have drifting EC. │
│ - Batch recipe deployment: push "Xà lách - Tuần 2" to all 16 stations. │
│ - Strict permission boundaries: shift workers can clean & top up,      │
│   but cannot alter core recipe targets or flash firmware.              │
│ - Audit log of every milliliter of acid and nutrient dosed.            │
├────────────────────────────────────────────────────────────────────────┤
│ FRUSTRATIONS & PAIN POINTS:                                            │
│ - Having to click through 16 individual station screens to find which  │
│   one has an abnormal pH drift.                                        │
│ - Shift workers accidentally pressing "Khôi phục cài đặt gốc" or      │
│   changing target EC without authorization.                            │
│ - Lack of yield tracking (kg harvested vs. liters of fertilizer dosed).│
├────────────────────────────────────────────────────────────────────────┤
│ SCENARIO (DAY IN THE LIFE):                                            │
│ Anh Thắng arrives at the greenhouse at 07:00 with an iPad. He opens   │
│ `/fleet` and sorts by "Cảnh báo trước". Station HG-04 shows a warning: │
│ pH 4.9 (below target 5.8). He checks the dosing history and sees pump  │
│ pH Up ran 3 times without raising pH, indicating an empty pH Up bottle.│
│ He assigns the morning technician to replace the reagent bottle.       │
├────────────────────────────────────────────────────────────────────────┤
│ UX & DESIGN IMPLICATIONS:                                              │
│ - Fleet View must be a primary top-level entry, not buried in Settings.│
│ - Strict role separation: Operator cannot invite users or reset station│
│   hardware; Viewer cannot resolve alerts or toggle pumps.              │
│ - Batch actions: "Áp dụng công thức cho N trạm" (`multi_device_template`)│
│ - Exportable CSV/PDF compliance reports for agricultural audits.       │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 6. End-to-End Crop Cycle Journey Map & IA Audit

Following `.agent/designer-skills/design-research/skills/journey-map/SKILL.md`, this journey map traces a complete 35-day crop cycle (e.g., Butterhead Lettuce) for Primary Persona **Minh**, cross-referenced against the current 5-tab Information Architecture (`Dashboard`, `Operations`, `Cultivation`, `Journal`, `Settings`).

### 6.1 Crop Cycle Journey Map Table

| Stage | User Goal | Actions & Behaviors | Touchpoints & Screens | Emotional State | Friction & Pain Points | Design Opportunities |
|---|---|---|---|---|---|---|
| **1. Unbox & Setup** | Assemble tower, connect station to home Wi-Fi, verify sensors. | Plugs in ESP32 nodes, scans QR code on station, selects home Wi-Fi SSID. | `Settings -> DevicePairing` (`/pairing`), `Settings -> Connectivity` | 😐 Apprehensive (-1) | QR scan requires manual fallback code; Wi-Fi configuration hidden inside Settings; no immediate feedback if sensors are submerged properly. | Guided onboarding wizard (`/interaction-design:design-onboarding`); automatic hardware self-test checklist. |
| **2. Recipe & Start** | Select lettuce recipe, fill water reservoir, start season. | Browses recipe library, applies targets (EC 1.4, pH 6.0), names season "Vụ Rau 1". | `Cultivation -> Recipes` (`/recipes`), `Cultivation -> Seasons` (`/crop-seasons`) | 😊 Excited (+2) | Recipe selection is detached from season creation; user must first apply recipe to device, then manually start season; unclear what "Misting ON 10s / OFF 180s" means. | Unified "Bắt đầu vụ mới" wizard that bundles recipe selection, tank fill confirmation, and season start in 3 steps. |
| **3. Daily Monitoring** | Verify plants are thriving; check daily water level & pH/EC. | Opens app once daily; glances at sensor cards; takes progress photo every Sunday. | `Dashboard` (`/dashboard`), `Cultivation -> Seasons -> PhotoJournal` | 😊 Reassured (+3) | Taking a photo requires leaving Dashboard, tapping Cultivation, finding active season, and opening photo drawer. | Add a quick-action "📸 Chụp ảnh tiến độ" FAB or tile directly on the Dashboard hero section. |
| **4. Alert & Anomaly** | React quickly when nutrient drifts or water depletes. | Receives push notification: "Mực nước thấp <15%"; opens app to confirm; refills water. | Push Notification, `Dashboard -> TankAlertBanner`, `Journal -> Events` (`/journal`) | 😰 Stressed (-2) | Push notification is currently absent on mobile (`useFCM.ts:20` bails out); alert text lacks plain-language action ("Châm thêm bao nhiêu lít nước?"). | Prescriptive alert recommendations: "Châm thêm 10L nước sạch để giảm EC về ngưỡng an toàn". |
| **5. Harvest & Close** | Collect mature lettuce, log final harvest weight, conclude cycle. | Harvests crop; taps "Kết thúc vụ"; records yield (e.g., 2.4 kg); views total days elapsed. | `Cultivation -> Seasons` (`/cultivation`), `Seasons -> History` | 🤩 Proud (+4) | No prompt to clean tank or recalibrate probes; yield weight field is missing in backend `crop_seasons` schema (`status` only). | Add yield (kg/grams) to season completion modal; trigger prompt: "Lưu vụ này thành công thức mẫu?". |
| **6. Clean & Repeat** | Sanitize reservoir and prepare next seedling batch. | Flushes water lines; runs pump purge; recalibrates pH probe with buffer. | `Operations -> Control` (`/operations`), `Settings -> Calibration` | 😐 Routine (0) | Calibration tools are buried 3 levels deep in Settings; manual pump flush requires toggling individual raw pump switches without a timed "Purge" routine. | "Vệ sinh bồn & Xả rửa" one-tap macro script; pH probe calibration wizard with stability detection. |

---

### 6.2 IA Audit: Structural Mismatches in the Current 5-Tab Navigation

```
CURRENT NAVIGATION HIERARCHY (MainLayout.tsx)
├── 1. Tổng quan (/dashboard)
│    └── Bento Cards, Health Score, Next Action, Emergency Button
├── 2. Vận hành (/operations)
│    ├── Tab: Điều khiển (Manual Pumps, PWM, E-Stop)
│    └── Tab: Tự động hóa (Visual Node Editor, Scripts)
├── 3. Canh tác (/cultivation)
│    ├── Tab: Mùa vụ (Active Season, Milestone, Photos)
│    ├── Tab: Công thức (Target EC/pH, Stages)
│    └── Tab: Lịch sử châm (Hourly Dosing Consumption)
├── 4. Nhật ký (/journal)
│    ├── Tab: Sự kiện (System Event Log, Acknowledge)
│    └── Tab: Phân tích (Analytics, Grafana Links)
└── 5. Cài đặt (/settings)
     ├── General, Thresholds, Dosing, Sensors, Connectivity
     └── [Orphaned Routes]: /pairing, /fleet, /config-backup, /roles
```

#### Discovered IA Mismatches & Cognitive Friction:
1. **Lịch sử châm (Dosing History) belongs to Operations/Logs, NOT Cultivation:**
   - *Current State:* Located under `Cultivation -> Lịch sử châm` (`Cultivation.tsx`, line 14).
   - *Problem:* Dosing history is a machine execution record (ml of chemical pumped per hour). Cultivation is a botanical lifecycle space (seasons, growth stages, photos). When an operator suspects an over-dosing incident, they naturally look in `Operations` or `Journal`, not in crop seasons.
2. **Operations merges Emergency Actuation with Complex Flow Authoring:**
   - *Current State:* `Operations` contains two tabs: `Điều khiển` (immediate pump toggles) and `Tự động hóa` (advanced ReactFlow node-graph authoring; `Operations.tsx`, lines 7–10).
   - *Problem:* High-risk juxtaposition. An operator trying to perform a rapid manual pump purge or emergency check is placed right next to a visual canvas editor for Rhai automation scripts.
3. **Fleet View is an Orphaned Sub-Page:**
   - *Current State:* `FleetView` lives at `/fleet` (`App.tsx`, line 95), accessible only via a link in `Settings -> GeneralSection.tsx`.
   - *Problem:* For any user managing more than one station, switching between stations requires navigating to `Settings -> General -> Fleet`, picking a station, and redirecting back. This violates mobile multi-station usability.
4. **Photo Journal Disconnected from Daily Workflow:**
   - *Current State:* Taking a crop photo is buried in `Cultivation -> Mùa vụ -> SeasonPhotoJournal.tsx`.
   - *Problem:* In single-station hobbyist use, taking a progress photo is the primary daily reward loop, yet it is completely absent from the main `Dashboard`.

---

## 7. Targeted User Interview Script (5 Archetypes)

This qualitative interview guide adheres to `.agent/designer-skills/design-research/skills/interview-script/SKILL.md`. It strictly avoids generic, leading questions ("Do you like the app?", "Was that easy?") and focuses on extracting concrete behavioral facts regarding the **3 Open Questions**.

### 7.1 Participant Recruitment Matrix (5 Target Archetypes)

| # | Archetype | Profile & Criteria | Primary Goal in Study |
|---|---|---|---|
| **P1** | **Urban Balcony Hobbyist** | Owns 1 home aeroponic/hydroponic system; grows for household consumption. | Validate single-station priority, notification urgency, and tolerance for technical complexity. |
| **P2** | **Micro-Farm Entrepreneur** | Operates 2–4 hydroponic towers/troughs; sells to local organic markets/restaurants. | Determine the exact tipping point where single-station UI breaks down and fleet view is required. |
| **P3** | **Commercial Greenhouse Supervisor** | Manages 10+ growing channels with 2–5 hired farm hands. | Clarify permission boundaries between supervisor (Admin) and shift workers (Operators/Viewers). |
| **P4** | **Hydroponic Agronomist / Consultant** | Formulates recipes; troubleshoots nutrient deficiencies for farm clients. | Evaluate recipe lifecycle, stage transitions, and telemetry data export needs. |
| **P5** | **Field Maintenance / Shift Worker** | Daily physical worker; cleans tanks, mixes fertilizer stocks, refills water. | Test day-to-day usability of manual controls, alert acknowledgment, and task safety locks. |

---

### 7.2 Structured Interview Guide (Duration: 45 Minutes)

#### Part 1: Welcome & Context (3 Minutes)
- *"Cảm ơn anh/chị đã dành thời gian tham gia buổi trao đổi hôm nay. Chúng tôi đang phát triển hệ thống điều khiển khí canh thông minh HYDRAGROW. Mục tiêu của buổi hôm nay không phải để đánh giá anh/chị, mà là để hiểu cách anh/chị đang vận hành thực tế giàn trồng của mình và những khó khăn thường gặp. Buổi trao đổi hoàn toàn mang tính nghiên cứu, không có câu trả lời nào là đúng hay sai."*

#### Part 2: Warm-Up & Current Workflow (7 Minutes)
1. *"Anh/chị hãy mô tả quy mô và mô hình giàn trồng hiện tại của mình (số lượng trạm, chủng loại cây, diện tích)?"*
2. *"Một ngày thông thường của anh/chị diễn ra như thế nào đối với việc chăm sóc và kiểm tra giàn trồng?"*
3. *"Anh/chị đang dùng những công cụ, thiết bị đo hoặc ghi chép nào để theo dõi dinh dưỡng và nước?"*

---

#### Part 3: Deep-Dive on Open Question (1) — Fleet vs. Single Station Priority (10 Minutes)
*Objective: Uncover whether the primary mental model is solitary station immersion or multi-station fleet monitoring.*

4. *(For P1/P2)* *"Khi kiểm tra giàn cây qua điện thoại, thông tin đầu tiên anh/chị muốn thấy ngay trong 3 giây đầu mở màn hình là gì?"*
   - *Probe:* *"Nếu anh/chị có 2 hoặc 3 trạm trồng khác nhau (ví dụ: trạm ươm xà lách và trạm dâu tây), anh/chị muốn xem từng trạm độc lập hay xem một màn hình tổng thể tình trạng của tất cả các trạm? Tại sao?"*
5. *(For P3)* *"Hãy kể lại một tình huống gần đây khi một trạm trong trang trại gặp sự cố dinh dưỡng hoặc cạn nước. Anh/chị đã phát hiện ra điều đó như thế nào giữa hàng chục trạm?"*
   - *Probe:* *"Khi cần điều chỉnh công thức hoặc lịch tưới cho nhiều trạm cùng một loại cây, anh/chị hiện đang làm thế nào? Điều gì tốn thời gian nhất trong thao tác đó?"*
6. *(Neutral Probing Guardrail):* Do NOT ask: *"Do you want a fleet view page?"*  
   Instead ask: *"Hãy miêu tả cách anh/chị phân bổ sự chú ý giữa các trạm trồng trong một buổi sáng làm việc?"*

---

#### Part 4: Deep-Dive on Open Question (2) — Role & Permission Boundaries (10 Minutes)
*Objective: Determine what actions a non-admin ("Viewer" vs. "Operator") must and must NOT be allowed to perform.*

7. *"Trong gia đình hoặc trang trại của anh/chị, có những ai khác cùng tiếp cận giàn trồng hoặc cài đặt ứng dụng trên điện thoại?"*
   - *Probe:* *"Họ cần xem những thông tin gì, và họ có quyền thao tác trực tiếp trên thiết bị (bơm, van, cài đặt) hay không?"*
8. *"Giả sử có một nhân viên mới hoặc người nhà hỗ trợ trông giàn cây khi anh/chị vắng mặt. Những thao tác nào anh/chị HOÀN TOÀN KHÔNG MUỐN người đó tự ý bấm vào trên ứng dụng, và hậu quả có thể là gì?"*
   - *Probe:* *"Nếu ứng dụng có thông báo cảnh báo đỏ (ví dụ: 'Lệch pH nghiêm trọng'), anh/chị có muốn người đó có quyền bấm 'Đã xử lý' để tắt cảnh báo không, hay chỉ người phụ trách mới được xác nhận?"*
9. *(Testing edge case from `api/control.rs` and `api/recipe.rs`):* *"Theo anh/chị, việc bấm nút Dừng Khẩn Cấp (E-stop) khi thấy ống nước vỡ nên dành cho bất kỳ ai nhìn thấy màn hình, hay chỉ người có quyền Quản trị?"*

---

#### Part 5: Deep-Dive on Open Question (3) — Real-time Alerting & Push Notifications (10 Minutes)
*Objective: Distinguish between critical alerts requiring loud push notifications and informative notifications that belong in an in-app log.*

10. *"Hãy kể lại lần gần nhất giàn trồng của anh/chị gặp sự cố nghiêm trọng (mất điện, cháy bơm, tràn bồn, cạn dinh dưỡng). Anh/chị đã biết chuyện đó vào lúc nào và bằng cách nào?"*
    - *Probe:* *"Khoảng thời gian từ lúc sự cố bắt đầu đến lúc anh/chị phát hiện ra là bao lâu? Nếu lúc đó có chuông báo trên điện thoại, anh/chị kỳ vọng chuông kêu như thế nào (thông báo im lặng, rung, hay báo động liên tục)?"*
11. *"Những thông báo nào sau đây anh/chị muốn ứng dụng GỬI NGAY vào điện thoại (dù điện thoại đang khóa trong túi), và những thông báo nào CHỈ CẦN HIỆN KHI MỞ ỨNG DỤNG?*
    - *Mực nước bồn còn dưới 20%*
    - *Độ pH lệch 0.4 so với mục tiêu*
    - *Mất kết nối Wi-Fi trạm 5 phút*
    - *Bơm vừa hoàn thành một mẻ châm dinh dưỡng 15ml*
    - *Cây đã chuyển sang giai đoạn sinh trưởng tiếp theo theo lịch"*
12. *"Anh/chị đang dùng điện thoại gì (iPhone hay Android)? Anh/chị có thường xuyên tắt quyền thông báo của các ứng dụng không, và lý do vì sao?"*

---

#### Part 6: Wrap-Up & Debrief (5 Minutes)
13. *"Nếu có một điều duy nhất mà ứng dụng này có thể làm để anh/chị hoàn toàn yên tâm đi công tác 1 tuần mà không lo giàn rau chết, điều đó sẽ là gì?"*
14. *"Còn điều gì quan trọng về cách anh/chị trồng và chăm sóc cây mà chúng tôi chưa hỏi đến không?"*
- *"Rất cảm ơn những chia sẻ thực tế và sâu sắc của anh/chị!"*

---

## 8. Quantitative Sensor Metrics Definition via InfluxDB

The high-fidelity specification (`docs/hydragrow-hifi-spec.md`) currently specifies arbitrary, ungrounded threshold values:
- Frame 14: *"Chậm 3 ngày so với công thức"* (Why 3 days? Based on what biological model?)
- Frame 16: *"Abnormal nutrient dosing >X% over Y hours"* (What are X and Y?)
- Frame 17: *"Collapse repeats: Mất/kết nối lại 5 lần trong 10 phút"*
- Backend default: `ec_ack_threshold = 0.05`, `ph_ack_threshold = 0.1`, `water_ack_threshold = 0.5` (`hydragrow-backend/migrations/20260312064526_init_schema.sql`, lines 124–126).

Relying on hard-coded heuristics causes either alert fatigue (false positives) or missed crop failures (false negatives). Below is the proposed quantitative methodology leveraging actual time-series telemetry in InfluxDB (`measurement: sensor_data`) and PostgreSQL dosing logs (`table: dosing_reports`).

> **Methodology Status & Database Access Audit:**  
> Direct connection to the local/staging InfluxDB instance was attempted (`http://localhost:8086/health`), but the service is offline/unreachable in this development environment. Consequently, while the statistical equations below (noise floor, EWMA, Z-score, consumption baselines) serve as the authoritative **mathematical methodology**, all specific numerical constants ($Y=4\text{ hours}$, $X=+50\%$, $Z \ge 2.5$, $Z \ge 4.0$, $0.5\text{ pH / 10 min}$) are **ASSUMPTIONS** based on physical heuristics. They **REQUIRE recalibration** against real InfluxDB/PostgreSQL data using the procedure in §8.3 prior to production deployment.

```
┌─────────────────────────────────────────────────────────────────────────┐
│              QUANTITATIVE SENSOR ANALYTICS PIPELINE                     │
├──────────────────────────┬──────────────────────────┬───────────────────┤
│ 1. Raw Telemetry Stream  │ 2. Statistical Baseline  │ 3. Dynamic Alerts │
│ (InfluxDB: EC, pH, Temp) │ (Rolling μ, σ, noise)    │ (Z-Score, EWMA)   │
├──────────────────────────┼──────────────────────────┼───────────────────┤
│ - Sampling: 1-5s         │ - Filter sensor ripple   │ - Dynamic window  │
│ - Dosing reports (ml)    │ - Stage baseline profile │ - Biological drift│
└──────────────────────────┴──────────────────────────┴───────────────────┘
```

### 8.1 InfluxDB Telemetry Data Model
As established in `hydragrow-backend/src/db/influx.rs` (lines 10–20), the time-series stream contains:
- **Bucket:** `sensors` (per `README.md`, line 23)
- **Measurement:** `sensor_data`
- **Tags:** `device_id`
- **Fields:**
  - `ec` (f64, mS/cm)
  - `ph` (f64, pH units)
  - `temp` (f64, °C water temperature)
  - `water_level` (f64, cm reservoir depth)
  - `ph_voltage_mv` (f64, raw millivolts from analog probe)

In PostgreSQL (`hydragrow-backend/migrations/20260504090000_add_dosing_reports_table.sql`):
- **Table:** `dosing_reports`
- **Columns:** `device_id`, `season_id`, `pump_a_ml`, `pump_b_ml`, `ph_up_ml`, `ph_down_ml`, `created_at`.

---

### 8.2 Methodology for Data-Driven Threshold Definition

#### Step 1: Quantify Sensor Hardware Noise Floor
Analog hydroponic probes in electrical water environments suffer from motor interference and RF ripple. We calculate the hardware noise floor per station:

$$\sigma_{\text{noise}} = \sqrt{\frac{1}{N} \sum_{i=1}^N (x_i - \bar{x})^2} \quad \text{during stable quiescent night hours (no pumps active)}$$

- *Empirical Calibration Rule:* Any alert threshold $\Delta_{\text{alert}}$ must satisfy:

$$\Delta_{\text{alert}} \ge 3 \cdot \sigma_{\text{noise}}$$

- *Illustrative Application:* If quiescent probe noise $\sigma_{\text{noise}} = 0.04\text{ pH}$, setting an alert threshold of $\pm 0.1\text{ pH}$ will trigger continuous false alarms. The threshold must dynamically scale above probe uncertainty.

#### Step 2: Dynamic Z-Score / EWMA Anomaly Detection for Chemical Drift
Rather than static bounds (e.g., $EC > 2.0$), compute an Exponentially Weighted Moving Average (EWMA) to detect sudden biological or actuator failures:

$$\hat{\mu}_t = \alpha \cdot x_t + (1 - \alpha) \cdot \hat{\mu}_{t-1}$$

$$Z_t = \frac{|x_t - \mu_{\text{target}}|}{\sigma_{\text{historical}}}$$

- **Warning Threshold Constant ($Z \ge 2.5$, $W = 15\text{ min}$):**  
  `ASSUMPTION (not yet run against actual InfluxDB data) — proposed initial value based on theoretical Gaussian 99% confidence interval (p ≈ 0.012) and the existing UI evaluation window default (ConditionGroupEditor.tsx, line 18); REQUIRES recalibration via actual queries before deployment to production.`
- **Critical Interlock Constant ($Z \ge 4.0$ or $|\frac{\Delta \text{pH}}{\Delta t}| > 0.5\text{ pH / 10 min}$):**  
  `ASSUMPTION (not yet run against actual InfluxDB data) — proposed initial value based on theoretical acid line rupture / siphon failure rates to prevent catastrophic root burn; REQUIRES recalibration via actual queries before deployment to production.`

#### Step 3: Dosing Anomaly Detection ($X\%$ over $Y$ hours)
Frame 16 states *"abnormal nutrient dosing >X% over Y hours"*. We mathematically ground $X$ and $Y$ using cumulative consumption curves from `dosing_reports`:

```flux
// Flux Query: Compute hourly fertilizer dosing distribution for device over past 14 days
from(bucket: "sensors")
  |> range(start: -14d)
  |> filter(fn: (r) => r["_measurement"] == "dosing_reports")
  |> filter(fn: (r) => r["device_id"] == "HG-0231")
  |> filter(fn: (r) => r["_field"] == "pump_a_ml" or r["_field"] == "pump_b_ml")
  |> aggregateWindow(every: 1h, fn: sum)
  |> quantile(q: 0.95)
```

- **Time Horizon Constant $Y$ ($4\text{ hours}$):**  
  `ASSUMPTION (not yet run against actual InfluxDB data) — proposed initial value based on theoretical aeroponic mixing, root absorption, and tank dilution cycles; REQUIRES recalibration via actual queries before deployment to production.`
- **Definition of $X$ (Threshold %):**

$$X = \left( \frac{\text{Dose}_{\text{actual, Y}} - \text{Dose}_{\text{median, Y, stage}}}{\text{Dose}_{\text{median, Y, stage}}} \right) \times 100\%$$

- **Dosing Anomaly Constant $X$ ($> +50\%$ over $P_{95}$):**  
  `ASSUMPTION (not yet run against actual InfluxDB data) — proposed initial value based on typical batch dosing variance heuristics; REQUIRES recalibration via actual queries before deployment to production.`

#### Step 4: Crop Growth Delay Metric ("Chậm tiến độ vụ")
Frame 14 states *"Chậm 3 ngày so với công thức"*. Plants do not mature by calendar days; they mature by cumulative physiological uptake:
1. **Cumulative Water & Nutrient Uptake:** In aeroponics, root volume is directly proportional to daily transpiration rate:

$$V_{\text{water, consumed}}(t) = \int_{0}^{t} \left(\text{Refill Rate} - \text{Evaporation Baseline}\right) dt$$

2. **Growth Progress Index (GPI):**

$$\text{GPI}(t) = \frac{\text{Actual Cumulative EC/Water Uptake from Day 0 to } t}{\text{Expected Cumulative Uptake according to Recipe at Day } t}$$

3. **Deviation Banner & Growth Delay Constants:**
   - **On-Track Band ($\text{GPI} \in [0.85, 1.15]$):**  
     `ASSUMPTION (not yet run against actual InfluxDB data) — proposed initial range reflecting ±15% natural biological variation; REQUIRES recalibration via actual queries before deployment to production.`
   - **Growth Delay Alert ($\text{GPI} < 0.75$ sustained for 3 consecutive days):**  
     `ASSUMPTION (not yet run against actual InfluxDB data) — proposed initial threshold replacing the ungrounded "3-day delay" in Frame 14; REQUIRES recalibration via actual queries before deployment to production.`

---

### 8.3 Calibration Procedure Upon Gaining Actual DB Access

When access to the production or staging InfluxDB instance (`bucket: sensors`) and PostgreSQL instance is provisioned, the following procedure must be executed without altering the underlying methodology:

1. **Responsible Roles:** Data / Agronomy Engineer or Backend Maintainer.
2. **Execution Trigger:** Before commissioning a new crop recipe or after the first 14 consecutive days of multi-station operational data collection.
3. **Actionable Query Suite:**

- **Query A: Calculate Station-Specific Noise Floor ($\sigma_{\text{noise}}$)**  
  Run against InfluxDB during quiescent night hours (01:00–04:00 local time, when pumps and lighting are stable):
  ```flux
  from(bucket: "sensors")
    |> range(start: -7d)
    |> filter(fn: (r) => r["_measurement"] == "sensor_data")
    |> filter(fn: (r) => r["_field"] == "ph" or r["_field"] == "ec" or r["_field"] == "temp")
    |> window(every: 1h)
    |> stddev()
    |> mean()
  ```
  *Calibration Action:* Set hardware alert minimum delta $\Delta_{\text{alert}} = \max(\text{ConfiguredTolerance}, 3 \times \sigma_{\text{noise}})$.

- **Query B: Compute Empirical Hourly Fertilizer Dosing Distribution ($P_{95}$)**  
  Run against PostgreSQL `dosing_reports` to establish true stage consumption baselines:
  ```sql
  SELECT 
      device_id,
      season_id,
      COUNT(*) AS total_dosing_events,
      ROUND(AVG(pump_a_ml + pump_b_ml)::numeric, 2) AS mean_hourly_fert_ml,
      ROUND(PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY (pump_a_ml + pump_b_ml))::numeric, 2) AS p95_hourly_fert_ml,
      ROUND(PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY (ph_up_ml + ph_down_ml))::numeric, 2) AS p95_hourly_ph_adjust_ml
  FROM dosing_reports
  WHERE created_at >= NOW() - INTERVAL '14 days'
  GROUP BY device_id, season_id;
  ```
  *Calibration Action:* Calibrate constant $X$ to flag when rolling consumption exceeds $1.5 \times P_{95}$ for that specific device and stage.

- **Query C: Compute Maximum Observed 10-Minute pH Derivative ($|\Delta\text{pH} / 10\text{m}|$):**  
  Run against InfluxDB:
  ```flux
  from(bucket: "sensors")
    |> range(start: -14d)
    |> filter(fn: (r) => r["_measurement"] == "sensor_data")
    |> filter(fn: (r) => r["_field"] == "ph")
    |> aggregateWindow(every: 10m, fn: spread)
    |> max()
  ```
  *Calibration Action:* Set Critical Emergency Shutoff threshold to $2 \times \max(\text{observed spread})$.

---

## 9. Authoritative Decisions & Status Table

The following table synthesizes the findings for the three foundational open questions from `docs/hydragrow-hifi-spec.md` (line 118) and confirms their current delivery state:

| Question # | Question Formulation | Current Repository Status | Concrete Evidence / Source Code Provenance | Actionable Next Step | Immediate Actionability |
|---|---|---|---|---|---|
| **(1)** | Is the priority a single-station household or a multi-station commercial fleet setup? | **Still an assumption / Requires interview & prod query** | - Backend schema natively supports multi-station: `device_ownership` uses `UNIQUE(user_id, device_id)` (`20260823100000_device_ownership.sql`, line 8).<br>- `/api/fleet/summary` exists (`hydragrow-backend/src/api/fleet.rs`, line 113).<br>- However, frontend IA is strictly 1-station scoped (`MainLayout.tsx`, lines 34–40; `useDeviceStore.ts`). `/fleet` is an orphaned sub-route.<br>- No live production database queries have been conducted to determine user proportions. | 1. Execute SQL queries 1–4 from §3 against the production database.<br>2. Conduct the 5 interviews outlined in §7 to evaluate willingness-to-pay and daily workflow priorities. | **BLOCKED on external data:** Depends on live production DB query and interview results. |
| **(2)** | What are the permission boundaries between "Operator" and "Viewer"? | **Partially specified in specs & code, but contains architectural leaks / Requires confirmation** | - Frontend defines role capabilities in `Roles.tsx` (lines 41–74).<br>- Backend encloses pump controls with `control:pump` and `control:emergency` (`api/control.rs`, lines 145, 401–410).<br>- **Discrepancy 1 (Security Bug):** In `api/alert.rs` (line 257), `resolve_event` requires only `read:telemetry`, allowing Viewers to dismiss alerts.<br>- **Discrepancy 2 (Security Bug):** In `api/recipe.rs` (line 561), `apply_recipe` only validates `is_owner`, neglecting to check `recipe:write` or Operator role.<br>- **Discrepancy 3:** In `Roles.tsx`, Operator UI says OTA/WiFi is Admin-only, but `ROLE_DEFAULT_SCOPES.operator` contains `device:ota` and `device:network`. | 1. Patch `api/alert.rs` to require `events:write` or `Operator` role for event acknowledgment.<br>2. Patch `api/recipe.rs` to enforce `recipe:write` scope on recipe application.<br>3. Align `Roles.tsx` capability definitions with backend scope checks. | **IMMEDIATE (Independent of interviews):** The two authorization bugs (`api/alert.rs` and `api/recipe.rs`) are verified source-code defects that can and should be fixed immediately without waiting for user discovery interviews. |
| **(3)** | Are FCM/APNs ready for actual deployment? | **Answered via code audit: Web FCM partially ready; Native Tauri & APNs NOT ready** | - **Backend:** Has Google OAuth2 FCM v1 pipeline (`hydragrow-backend/src/services/fcm.rs`, lines 21–70), but depends on untracked `firebase-service-account.json`.<br>- **Frontend Web:** Registers web tokens via `useFCM.ts` (`requestForWebToken()`).<br>- **Native Mobile (Tauri):** `useFCM.ts` (lines 19–23) explicitly bails out: `if (!isWeb) return;`. Native push is completely unhandled.<br>- **APNs:** Zero Apple Push Notification code, zero APNs tokens table, zero iOS provisioning credentials. | 1. Provision production `firebase-service-account.json` on deployment server.<br>2. Integrate Tauri Native notification/push plugin for Android & iOS.<br>3. For iOS/APNs, configure FCM APNs payload or direct Apple APNs HTTP/2 client. | **ACTIONABLE technical task:** Ready for infrastructure configuration and native plugin implementation; no further user research needed to establish unreadiness. |

---

## 10. Conclusion & Immediate Recommendations

1. **Do not begin building new UI features or restructuring code** until the 5 user interviews (§7) and the production database queries (§3) have established whether HYDRAGROW's initial target market is **Minh (Single-Station Household)** or **Anh Thắng (Multi-Station Commercial)**.
2. **Prioritize fixing the security scope leaks** identified in §9 (closing Viewer alert resolution in `api/alert.rs` and enforcing `recipe:write` in `api/recipe.rs`).
3. **Refactor the 5-tab Navigation** when entering the next design cycle: relocate `Dosing History` from `Cultivation` into `Journal/Operations`, elevate `Fleet View` into top-level navigation if multi-station adoption is $>20\%$, and place a fast "Photo Journal" action on the main Dashboard.
4. **Transition from arbitrary thresholds to InfluxDB statistical models** (§8) during the next supervisor worker release (`hydragrow-diagnostic-worker`).

## 11. Competitive Analysis

**Purpose:** Sections 1–10 established three JTBD tracks (§4) from internal evidence (code, schema, spec) alone. This section benchmarks each track against real, currently-marketed products and platforms, using only sources with a verifiable public link. Per the governing constraint on this section, no product name, feature claim, or price is stated unless a live public source was found for it; where public information was insufficient, that is stated explicitly rather than estimated. All sources were retrieved 2026-09-12 and reflect the state of each vendor's public materials on that date; consumer hydroponics is a volatile market (see 11.1) and prices/features may have changed since.

---

### 11.1 Track A — "Peace of Mind / Hobby" vs. Consumer Hydroponic Products

**Products reviewed:** AeroGarden, Click & Grow, Rise Gardens. (A dedicated search for a Vietnamese consumer-market equivalent — e.g., a commercially sold, app-connected home hydroponic unit — did not surface one; results returned only DIY/academic IoT hydroponics projects from Vietnamese universities and generic industrial monitoring apps repurposed for aquaculture/irrigation. **Insufficient public sources; requires independent research** to confirm whether a Vietnamese consumer-grade equivalent exists.)

| Product | Remote monitoring / "peace of mind" | Push alerts | Multi-user / shared accounts | Notes on source |
|---|---|---|---|---|
| **AeroGarden** (app-connected Bounty/Farm models) | WiFi-connected app shows light status and reports water/nutrient state; company underwent a 2024 shutdown announcement, a 2025 relaunch under new ownership, and is operating again as of 2026 with the same app model. [AeroGarden FAQs](https://scottsmiraclegro.com/en-us/support/help-center/aerogardenfaqs.html); [AeroGarden Review 2026](https://greenerpods.com/aerogarden-review-2026-still-worth-buying-after-shutdown/) | Yes — "Add Water," "Add Nutrients," "Reorder Supplies," "Gardening Tips" push categories, individually toggleable. [AeroGarden alerts FAQ](https://www.aerogarden.com/blog/tag/alerts/) | No evidence of a multi-user / shared-household account model; account is tied to a single login, and forum reports describe notification reliability issues rather than any sharing feature. [AeroGarden Bounty forum](https://aerogardenaddicts.com/thread/2552/bounty) | High confidence (official support docs + user forum) |
| **Click & Grow** (Smart Garden 9 Pro / Click & Grow 25) | Bluetooth-paired app shows moisture-sensor-driven watering status and light-cycle control. [App Store listing](https://apps.apple.com/it/app/id1451077111) | Yes — low-water notifications reported by users, though push delivery reliability is inconsistent (one review: only 1 of 8 plants triggered a notification). [App Store reviews via SplitMetrics](https://splitmetrics.com/apps/click-grow-official/id1451077111) | No evidence of multi-user/shared-household accounts; single-account model with reported sign-in/session bugs. [App Store reviews](https://splitmetrics.com/apps/click-grow-official/id1451077111) | Medium confidence (official listing + aggregated reviews) |
| **Rise Gardens** | App-guided "Smart Care" monitors water level and plant progress; explicitly designed, per founder, so people don't have to feel "tethered" to the garden. [TechCrunch](https://techcrunch.com/2021/07/19/rise-gardens-grows-with-9m-series-a-to-help-anyone-be-an-indoor-farmer/) | Yes — documented push categories for low water, very-low water (with pump shutoff), and high water, each on a fixed re-notify cadence. [Rise Gardens support: Creating Your Account](https://support.risegardens.com/en_us/creating-your-account-Hym7XT1w0) | No — support documentation explicitly instructs the buyer to create the account **with the same email/password used for the original purchase**, i.e. a single-owner account model, not a multi-user or role-based one. [Rise Gardens support](https://support.risegardens.com/en_us/creating-your-account-Hym7XT1w0) | High confidence (official support article) |

**Gap analysis — what HYDRAGROW lacks vs. Track A competitors:**
1. **Working native push, today.** All three competitors ship functioning (if occasionally flaky) push notifications as a baseline, out-of-the-box feature. Per this document's own §9 (Question 3), HYDRAGROW's native mobile (Tauri) push path is explicitly unimplemented (`if (!isWeb) return;`) and APNs is entirely absent. On the single dimension most central to Track A's job statement ("notify immediately if urgent intervention is needed"), HYDRAGROW is currently behind, not ahead of, the low-end hobby competition.
2. **Zero-configuration onboarding.** All three competitors are closed consumer appliances with companion apps that require no self-hosted infrastructure. HYDRAGROW's stack (MQTT broker, InfluxDB, PostgreSQL, Firebase project) is not something a "Minh" persona (single-station household hobbyist) can stand up alone; a real hobbyist buys a box, not a Rust/React monorepo.
3. **Brand/support continuity risk is a competitor problem, not a HYDRAGROW gap** — AeroGarden's 2024 shutdown-then-relaunch is a cautionary tale for the category, not a HYDRAGROW weakness.

**What HYDRAGROW does NOT lack relative to this track:** none of the three reviewed products offer multi-user or role-based accounts of any kind — every one of them is architected around a single owner login. HYDRAGROW's Operator/Viewer/Admin schema (§2.3) is a capability this entire competitive tier simply does not have. This means the "Viewer" role is not something Track A customers are asking for or comparing against; it is a Track B (commercial) capability bolted onto a Track A (hobbyist) product.

---

### 11.2 Track B — "Commercial Ops" vs. Commercial Greenhouse Control Systems

**Products reviewed:** Priva (Connected/Access Control), Argus Controls (TITAN and its successor, Axia), Autogrow (MyAutogrow/AG-Insights), Growlink.

| Product | Operator/Viewer-equivalent permissions | Multi-station / multi-site management | Public price | Source |
|---|---|---|---|---|
| **Priva** | Far more granular than a binary Operator/Viewer split: permissions are broken out **per application** (Access Control, Building/Cloud Operator, Notification Center, Analytics, Provisioning, etc.), each independently flagged as *organization-wide*, *site-specific*, or *fine-grained* (sub-building level), plus three numbered edit levels (daily setpoints → maintenance settings → control-behavior changes) roughly analogous to Operator vs. Admin. Multi-organization "site sharing" lets one org grant scoped access to another org's sites. | Native multi-site: audit logs, notification rules, and site-sharing are all explicitly scoped per-site or org-wide, with users able to hold different permission sets on different sites simultaneously. | **Not publicly listed** — enterprise/quote-based. **Insufficient public sources; requires independent research** (direct vendor quote) to compare against HYDRAGROW cost. | [Priva user permissions doc](https://support.priva.com/hc/en-us/articles/360016228560-Explanation-of-user-permissions-Building-Automation) |
| **Argus Controls** | Argus's own vendor comparison of its legacy **TITAN** system vs. its newer **Axia** platform states TITAN has only "basic user setup and permissions" with multi-user support "supported with limitations," while Axia adds "advanced role-based permissions with tiered access across organizations" and a "fully optimized multi-user environment." | TITAN: "sites managed individually or with added complexity." Axia: "centralized multi-site access and control in one interface." | Not publicly listed (call-for-price / dealer quote in all listings found). **Insufficient public sources; requires independent research.** | [Argus TITAN vs. Axia comparison PDF](https://arguscontrols.com/uploads/documents/TITAN-vs.-Axia-MK0356-Rev-00.pdf) |
| **Autogrow (MyAutogrow / AG-Insights)** | Documentation states "you choose who can access your controller" and supports adding outside consultants, but no published breakdown of discrete permission levels (e.g., no explicit Operator-vs-Viewer split found in public docs). | Cloud remote-access add-on for the MultiGrow controller; framed around "your facility," not explicitly multi-site in the public material reviewed. | **Published:** US$50/device/month for remote access + alerts (MyAutogrow); US$100/device/month for the analytics tier (AG-Insights), billed annually. | [Autogrow MyAutogrow pricing](https://autogrow.com/our-products-solutions/myautogrow) |
| **Growlink** | Explicit tiering: the free "Root" and $25/month "Sprout" hobby plans include **1 user only** with no permissions feature; the commercial "Bloom"/"Harvest" tiers (Harvest = $1,000/month per facility) include up to 20 bundled users and list "User permissions" as an **included, gated commercial feature** (not available on the hobby tiers at any price); additional users can be added a la carte ($10–$20/user/month depending on tier). | Explicitly multi-facility on the commercial tiers ("Harvest — Multi-facility, dedicated support"). | **Published, tiered:** Free → $25/mo → (mid-tier "Bloom," price not shown in the fetched page) → $1,000/mo/facility, plus itemized add-ons for sensors, users, and AI features. | [Growlink pricing page](https://shop.growlink.com/pricing.html) |

**Gap analysis — what HYDRAGROW lacks vs. Track B competitors:**
1. **Permission granularity.** HYDRAGROW currently has exactly two meaningful roles in practice (Operator, Viewer; Admin for provisioning) with a flat, all-or-nothing scope model. Priva breaks permissions out per-application and per-site with three sub-levels of edit access; Argus explicitly sells "tiered, cross-organization RBAC" as the flagship upgrade differentiating its new platform from its old one. Commercial buyers evaluating HYDRAGROW against Argus Axia or Priva would be comparing a 2-role system to an N-dimensional one.
2. **The two authorization bugs already identified in §9** (`api/alert.rs` allowing Viewers to resolve alerts; `api/recipe.rs` not enforcing `recipe:write`) are not just internal defects — they are exactly the class of gap that would disqualify HYDRAGROW in a commercial RFP against vendors who sell RBAC rigor as core IP.
3. **No native multi-organization site-sharing.** Priva's ability to grant another company scoped access to your site (e.g., an agronomy consultant, a franchise HQ) has no counterpart described anywhere in the reviewed HYDRAGROW schema.
4. **Price positioning is genuinely favorable, but not for the reason assumed.** §4.2's assumption that "industrial systems are prohibitively expensive ($10k+)" is not fully supported by what is public: Autogrow's and Growlink's *cloud/software* layers are $50–$1,000/month, not a $10k+ capital outlay (though Priva/Argus's underlying hardware and installation likely still carry the large capex this document assumed — that hardware pricing remains unconfirmed publicly). HYDRAGROW's real cost advantage is that it is self-hosted software with no subscription fee at all — but that advantage comes with an offsetting cost the competitors absorb for their customers: HYDRAGROW's owner must run and maintain their own MQTT/InfluxDB/PostgreSQL/Firebase stack, which none of these vendors require of their customers.

---

### 11.3 Track C — "Agronomic R&D" vs. Research/Datalogging Tools

**Products/approaches reviewed:** TrolMaster Hydro-X/Aqua-X (with TM+ Pro app), Onset HOBO dataloggers, Campbell Scientific dataloggers, and a documented independent researcher's InfluxDB+Grafana stack.

| Tool | What it offers | Data export / correlation analysis | Notes |
|---|---|---|---|
| **TrolMaster Hydro-X / Aqua-X + TM+ Pro app** | Central environmental + irrigation controller; in-app "Historical Chart" combining all sensor history into one graph, a "Logbook" for manually tagging plant stages/feed events, and controller-sharing so an owner can authorize other accounts on the same controller. | Data lives inside TrolMaster's own app/cloud; the public materials reviewed describe in-app charting and logbook tagging but **no CSV/API export or open data path** for external statistical correlation was found in the sources reviewed. | [TM+ Pro app listing](https://apps.apple.com/app/id1619222131); [TrolMaster Hydro-X product page](https://hydrobuilder.com/collections/trolmaster-hydro-x-series?page=5); [TrolMaster Aqua-X product page](https://hydrobuilder.com/collections/trolmaster-aqua-x-series?page=4) |
| **Onset HOBO / Campbell Scientific dataloggers** | Purpose-built, high-accuracy standalone loggers for temperature, RH, PAR, soil moisture, water level, pH, EC, etc.; widely used in agricultural/ecological research. Prices range roughly **US$85–$2,500 per unit** depending on sensor type and logging capability. | Bluetooth/cabled offload into proprietary desktop software (e.g., Onset's HOBOware) for later analysis; not natively a live dashboard/alerting system like InfluxDB+Grafana, and correlating dosing events with sensor data requires manual merging of exported files. | [HOBO/Onset pricing (Tequipment)](https://www.tequipment.net/HOBO-by-Onset/series_mx2300-series); [Datalogger cost comparison, UBC](https://www.eoas.ubc.ca/courses/atsc303/Labs/2023/datalogger-info_2023/datalogger-comparison-2023.pdf) |
| **Independent researcher InfluxDB + Grafana stack** (documented case study, not a commercial product) | A named practitioner explicitly abandoned his hardware vendor's bundled "black-box" sensor app — citing no data ownership and "no path to real analysis" — and rebuilt the same class of system HYDRAGROW already runs: sensors writing to InfluxDB, visualized in Grafana, self-hosted. | This is the exact architecture already used by HYDRAGROW (`hydragrow-backend`, InfluxDB `sensors` bucket, §8's Flux queries). | [InfluxData webinar description](https://www.influxdata.com/resources/from-black-box-sensor-app-to-research-grade-monitoring-greenhouse-iot-stack-with-influxdb/) |

**Gap analysis — what HYDRAGROW lacks vs. Track C tools:**
1. **Not an architecture gap — a maturity gap.** The InfluxDB/Grafana case study is direct, dated (2026) evidence that agronomically-minded users are moving *toward* HYDRAGROW's existing stack, specifically because closed commercial platforms like TrolMaster don't expose raw, exportable time-series for statistical work. HYDRAGROW is not behind Track C's tooling curve architecturally.
2. **Where HYDRAGROW is behind:** the constants problem already flagged in §8 — HYDRAGROW's alert/deviation thresholds are explicitly uncalibrated assumptions pending real data (§8.3), whereas TrolMaster ships a working (if closed) historical-chart-and-logbook UI today, and standalone dataloggers ship with vendor-validated accuracy specs out of the box. An agronomist choosing between "a correct architecture with unvalidated thresholds" and "a validated but closed commercial tool" may reasonably pick the latter until §8.3's calibration is executed.
3. **No pre-built analysis/dashboard layer.** Nothing in the reviewed HYDRAGROW code/docs suggests pre-built Grafana dashboards or statistical templates are shipped to the end user; the researcher case study and HOBO/Campbell workflows both still require the user to do their own dashboard/analysis construction — so on this specific point HYDRAGROW is at parity with, not behind, the DIY end of this track, but is behind a turnkey product like TrolMaster's Logbook for a non-programmer agronomist.

---

### 11.4 Synthesis — Implications for Open Question (1)

Open question (1) in §9 asks whether HYDRAGROW's priority should be a single-station household product or a multi-station commercial fleet product. The competitive evidence gathered above bears directly on that question, independent of the interview/query results §9 already calls for:

1. **Tracks A and B are not the same market, and no company reviewed serves them with one undifferentiated product.** AeroGarden, Click & Grow, and Rise Gardens are consumer appliance companies with zero commercial-grade permissioning; Priva and Argus are enterprise automation vendors with zero presence in the consumer hobby space; Autogrow targets established growers, not apartment hobbyists. **Growlink is the one vendor found that spans both segments — and it does so by explicit tiering, not a single flat interface:** its free/$25 hobby tiers cap at 1 user with permissions unavailable at any price, while its $1,000/month "Harvest" commercial tier bundles 20 users and gates "user permissions" as a paid, commercial-only feature. Even the one company straddling both worlds treats "who else can log in and what can they do" as a paid, commercial-tier differentiator rather than a universal feature.
2. **This directly informs the risk framed in the prompt for this task.** HYDRAGROW's current IA (§2.3, §9) exposes the same Operator/Viewer/Admin role system to every user regardless of whether they are Minh (single-station hobbyist, who Track A evidence shows doesn't want or need role management) or Anh Thắng (multi-station commercial operator, who Track B evidence shows needs role management an order of magnitude more granular than what exists today). Building one flat interface for both is not what any reviewed competitor — including the single vendor that serves both segments — has chosen to do.
3. **Refinement to open question (1):** the evidence suggests the binary framing of Q1 ("household OR commercial") may itself be the wrong shape of the question. The more actionable version, informed by Growlink's model, is: *should HYDRAGROW adopt a tiered product structure* (one shared backend/data model — which Track C's evidence suggests is already sound — with a deliberately simplified hobbyist-facing IA and a deliberately more granular, Priva/Argus-style permission and multi-site IA for commercial accounts), *rather than a single undifferentiated interface serving both personas identically?* This reframing should be added to the §7 interview script as an explicit question to both the "Minh" and "Anh Thắng" archetypes, and the fleet/permission-focused SQL queries in §3 should be extended to check whether any current accounts are already mixing single-station and multi-station usage patterns under one login — which would be early internal evidence for or against the tiering hypothesis.
4. **Caveat on evidence strength:** this analysis reviewed 2–4 named products per track from public marketing/support pages, not licensed analyst market-sizing data, and the consumer segment in particular is volatile (AeroGarden's 2024 shutdown and 2025 relaunch occurred within the lookback window). This section's conclusions should be treated as a directional risk flag for the §7 interviews to test, not as a substitute for them.
