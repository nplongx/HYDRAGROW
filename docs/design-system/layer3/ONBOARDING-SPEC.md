# Onboarding Experience Specification (Track D)

**Document Status:** Approved / Spec  
**Target:** 
- `hydragrow-frontend/src/components/onboarding/OnboardingWizard.tsx`
- `hydragrow-frontend/src/components/onboarding/OnboardingStep.tsx`
- `hydragrow-frontend/src/components/onboarding/index.ts`
- `hydragrow-frontend/src/hooks/useOnboardingState.ts`

**Applied Skills:** `onboarding-design`, `peak-end-rule`, `form-design`, `aesthetic-usability`, `feedback-patterns`

---

## 1. Executive Summary & Problem Context

Currently, `hydragrow-frontend` offers **zero guided onboarding**. When a grower signs up via Firebase Auth, they land directly on `Dashboard.tsx` facing:
- Empty sensor cards displaying default or zeroed telemetry (`--.-`, `Offline`).
- An empty quick action bar with disabled or non-contextual actions.
- No immediate next steps or orientation towards connecting physical hardware.

In hydroponics/aeroponics, system confidence depends on trusting real-time automation. An empty dashboard creates uncertainty ("Is the controller broken? Did my Wi-Fi fail?").

### The "Aha Moment"
> **The Aha Moment:** The exact instant live telemetry (EC, pH, temperature, water level) pulses onto the screen from the grower's newly-paired hardware station.

Seeing the initial sensor reading transforms an abstract IoT dashboard into a living, responsive cultivation partner.

---

## 2. Onboarding Principles (from `onboarding-design` & `peak-end-rule`)

1. **Get to Value Fast:** Compress the path from account creation to the first live sensor packet. Avoid upfront configuration quizzes or profile embellishments.
2. **Orient, Don't Educate:** Guide the user to where actions happen (e.g. `/pairing`, `/seasons`) rather than lecturing them on nutrient chemistry or firmware nuances.
3. **Build Confidence with Early Wins:** Each step completed produces positive, visible feedback (green checkmarks, progress increment).
4. **Reduce Setup Friction:** The wizard is an unobtrusive dashboard banner card (dismissible at any time), not an inescapable modal barrier.
5. **Peak-End Rule & Celebration:** When all steps conclude, reward the grower with a celebratory completion state ("🎉 Hệ thống đã sẵn sàng! Chúc vụ mùa bội thu.") reinforcing high emotional satisfaction before transitioning permanently to active operations.

---

## 3. The 4-Step Progressive Onboarding Funnel

The wizard renders as a structured vertical checklist embedded directly in the Dashboard layout:

| Step # | Step ID | Title (VI) | Description (VI) | Primary CTA | Auto-trigger / Condition |
|---|---|---|---|---|---|
| **1** | `welcome` | **Chào mừng đến HydraGrow** | Nền tảng giám sát và điều khiển khí canh thông minh. Khám phá cách tự động hóa trang trại của bạn trong 4 bước đơn giản. | "Bắt đầu ngay" (Advances to Step 2) | Manual tap on CTA |
| **2** | `pair_device` | **Kết nối thiết bị** | Ghép nối trạm cảm biến và bộ điều khiển đầu tiên qua mã QR hoặc địa chỉ IP mạng nội bộ. | "Ghép nối thiết bị" (Navigates to `/pairing`) | Manual navigation or auto-detected when `firstDevicePaired` becomes true |
| **3** | `first_data` | **Dữ liệu thời gian thực** | Khi trạm online, bạn sẽ thấy chỉ số EC, pH, nhiệt độ và mực nước cập nhật trực tiếp tại đây. | "Đang chờ dữ liệu..." / "Xem bảng điều khiển" | **Auto-completes** when `useDeviceStore.sensorData` receives telemetry |
| **4** | `first_season` | **Bắt đầu mùa vụ** | Thiết lập công thức dinh dưỡng và chu kỳ phun sương phù hợp với loại cây trồng đầu tiên của bạn. | "Tạo mùa vụ mới" (Navigates to `/seasons`) | Navigates to crop season manager |

---

## 4. State Machine & Persistence (`useOnboardingState`)

### 4.1 Storage Schema
Persisted under `localStorage` key: `hydragrow_onboarding`

```ts
export interface OnboardingState {
  completedSteps: string[];
  dismissed: boolean;
  firstDevicePaired: boolean;
  firstDataReceived: boolean;
  firstSeasonCreated: boolean;
}
```

### 4.2 Default Initial State
```json
{
  "completedSteps": [],
  "dismissed": false,
  "firstDevicePaired": false,
  "firstDataReceived": false,
  "firstSeasonCreated": false
}
```

### 4.3 Hook API (`useOnboardingState`)
- `completedSteps: string[]`
- `dismissed: boolean`
- `isStepComplete: (stepId: string) => boolean`
- `completeStep: (stepId: string) => void`
- `dismiss: () => void`
- `reset: () => void`
- `shouldShowOnboarding: boolean` (Evaluates to `true` if `!dismissed && completedSteps.length < 4`)
- `currentStepIndex: number` (Calculates active focus index based on first incomplete step)
- `progressPercentage: number` (`(completedSteps.length / 4) * 100`)

---

## 5. Component Specifications

### 5.1 `OnboardingStep.tsx`
A reusable row representing one step of the progressive wizard.

- **Props:**
  - `title: string`
  - `description: string`
  - `icon?: React.ReactNode`
  - `primaryAction?: { label: string; onClick: () => void }`
  - `secondaryAction?: { label: string; onClick: () => void }`
  - `isComplete?: boolean`
  - `isActive?: boolean`
  - `stepNumber?: number`

- **Visual States:**
  - **Complete (`isComplete = true`):**
    - Step badge/icon shows green circle check (`CheckCircle2` / `--color-status`).
    - Title is muted, body text subdued.
    - Actions hidden or replaced with subtle "Đã hoàn thành" indicator.
  - **Active (`isActive = true`):**
    - High-contrast border (`border-primary`), slight elevation or pill background accent.
    - Prominent primary CTA button (`ui-btn-primary`, `--color-primary-deep`).
  - **Pending / Upcoming:**
    - Clean border (`border-line`), step number indicator, neutral text.

### 5.2 `OnboardingWizard.tsx`
Card component embedded within the dashboard.

- **Layout Structure:**
  - **Header:**
    - Icon badge (`Sparkles` or `Sprout`).
    - Headline: "Thiết lập hệ thống HydraGrow" with step counter badge (e.g., `2/4 hoàn thành`).
    - Progress Bar (`h-1.5 w-full bg-line rounded-full overflow-hidden` with `bg-primary` fill).
    - Dismiss Button: "Bỏ qua hướng dẫn" / `X` button (calls `dismiss()`).
  - **Step List:** Vertical list of 4 `OnboardingStep` components.
  - **Reactive Telemetry Watcher:** Automatically hooks into `useDeviceStore((s) => s.sensorData)`. If valid sensor data is received and `first_data` is not yet completed, triggers `completeStep('first_data')`.
  - **Peak-End Celebration Banner:**
    - Triggered when all 4 steps are marked complete.
    - Features: Warm celebration banner (`bg-pill border border-status/30 text-primary-deep`), celebratory emoji (`🎉`), congratulatory copy ("Hệ thống đã sẵn sàng! Chúc vụ mùa bội thu."), and a button "Hoàn tất" to dismiss the wizard permanently.

---

## 6. Empty State Improvements for Unpaired Stations

To prevent cognitive dissonance when new growers see empty dashboards:
1. **Sensor Bento Cards:** When `sensorData === null`, display a friendly prompt:
   - "Chưa có tín hiệu cảm biến — Hãy kết nối trạm của bạn để bắt đầu đọc dữ liệu tự động."
2. **Fleet View:** When `ownedDevices.length === 0`:
   - Display full-screen illustration empty state prompting `/pairing`.
3. **Quick Action Bar:** Disable manual dosing triggers until at least one station is online, offering an inline tooltip "Cần kết nối trạm trước khi điều khiển bơm."

---

## 7. Metrics & Analytics Framework

| Metric | Definition | Goal / Benchmark |
|---|---|---|
| **Activation Rate (Device Paired)** | % of new signups who complete `pair_device` within 24 hours | > 75% |
| **Time to First Telemetry (TTFT)** | Median duration from signup to first `sensorData` frame received | < 15 minutes |
| **Onboarding Completion Rate** | % of users who reach all 4 completed steps | > 60% |
| **Drop-off Point** | Step with highest abandonment | Monitored to optimize QR pairing UX |
| **Day-7 Retention** | % of activated users returning to Dashboard at Day 7 | > 50% |

---

## 8. Q1 Extension Points (Household vs. Commercial)

Defensively architected to bifurcate smoothly when user research is finalized:

1. **Persona Selection Hook (`onboarding_persona`):**
   - **Hobbyist / Household ("Vườn ban công / Trong nhà"):**
     - Simple 1-station flow.
     - Pre-configured preset recipes ("Rau muống thủy canh", "Xà lách Romaine").
     - Automated default alerts.
   - **Commercial Farm ("Trang trại thương mại / Nhà màng"):**
     - Multi-station batch provisioning wizard.
     - Organization & Zone setup step.
     - Role assignment prompt (`Admin`, `Operator`, `Agronomist`).
2. **Admin-Only Station Provisioning Mode:**
   - Dedicated flow for technician/installer accounts bypassing consumer onboarding to bulk flash and register hardware IDs.
