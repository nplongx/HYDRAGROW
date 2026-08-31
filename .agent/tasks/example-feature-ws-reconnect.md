# [Jules Feature] frontend: Add WebSocket reconnect with exponential backoff

> **Ví dụ task feature hoàn chỉnh.**  
> Dispatch: `./scripts/jules task .agent/tasks/example-feature-ws-reconnect.md`

## Task type: Feature — 3-phase protocol

**Subsystem:** `frontend`  
**Scope:** `hydragrow-frontend/src/` (WebSocket client layer)  
**PR prefix:** `feat/ws-reconnect-backoff`

---

## Acceptance criteria (binary scoreable)

- [ ] Sau khi mất kết nối, client tự reconnect với exponential backoff: 1s → 2s → 4s → 8s → 16s (max)
- [ ] Retry count hiển thị trên UI (status indicator)
- [ ] Sau 5 lần fail: toast thông báo "Mất kết nối — đang thử lại..."
- [ ] Khi reconnect thành công: toast "Đã kết nối lại"
- [ ] Unit test coverage: 80% trên reconnect logic
- [ ] Không regression: existing WebSocket tests vẫn pass

---

## Phase 1 — Discovery (viết KHÔNG code)

- [ ] Đọc: `docs/superpowers/specs/module-rules/frontend.md`
- [ ] Locate WebSocket client hiện tại: `grep -r "WebSocket\|useWebSocket\|ws://" src/ -l`
- [ ] Xác định Zustand store cho connection state
- [ ] Xác định component nào hiển thị connection status
- [ ] Comment: `Phase 1 complete — files to change: <list>`

## Phase 2 — Test first

```typescript
// Viết tests trước khi implement
// File: hydragrow-frontend/src/__tests__/wsReconnect.test.ts

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { createReconnectStrategy } from '../lib/wsReconnect'

describe('WebSocket reconnect strategy', () => {
  it('uses exponential backoff: 1s, 2s, 4s, 8s, 16s', () => {
    const strategy = createReconnectStrategy({ baseDelay: 1000, maxDelay: 16000 })
    expect(strategy.nextDelay(0)).toBe(1000)
    expect(strategy.nextDelay(1)).toBe(2000)
    expect(strategy.nextDelay(2)).toBe(4000)
    expect(strategy.nextDelay(3)).toBe(8000)
    expect(strategy.nextDelay(4)).toBe(16000)
    expect(strategy.nextDelay(5)).toBe(16000) // capped
  })

  it('resets delay after successful connection', () => {
    const strategy = createReconnectStrategy({ baseDelay: 1000, maxDelay: 16000 })
    strategy.recordFailure()
    strategy.recordFailure()
    strategy.reset()
    expect(strategy.nextDelay(0)).toBe(1000)
  })

  it('emits onMaxRetries after 5 failures', () => {
    const onMax = vi.fn()
    const strategy = createReconnectStrategy({ maxRetries: 5, onMaxRetries: onMax })
    for (let i = 0; i < 5; i++) strategy.recordFailure()
    expect(onMax).toHaveBeenCalledOnce()
  })
})
```

- [ ] Chạy: `cd hydragrow-frontend && npx vitest run src/__tests__/wsReconnect.test.ts`
- [ ] Xác nhận: FAIL (function chưa tồn tại)

## Phase 3 — Implement

### Files cần tạo/sửa

| Action | File | Mô tả |
|--------|------|-------|
| Create | `src/lib/wsReconnect.ts` | Pure reconnect strategy (no React deps) |
| Modify | `src/hooks/useWebSocket.ts` | Tích hợp strategy vào hook |
| Modify | `src/store/connectionStore.ts` | Thêm `retryCount`, `isReconnecting` state |
| Modify | `src/components/StatusBar.tsx` | Hiển thị retry state |

### Implement order

1. `src/lib/wsReconnect.ts` — pure logic, dễ test nhất
2. Tests pass: `npx vitest run src/__tests__/wsReconnect.test.ts`
3. `src/store/connectionStore.ts` — thêm state mới
4. `src/hooks/useWebSocket.ts` — tích hợp
5. `src/components/StatusBar.tsx` — UI
6. Full test suite: `npx vitest run`

### Verification

```bash
cd hydragrow-frontend
npx tsc --noEmit      # 0 type errors
npx eslint .          # 0 lint errors
npx vitest run        # all pass, new tests pass
npm run build         # build thành công
```

### Commit sequence

```bash
git add src/lib/wsReconnect.ts src/__tests__/wsReconnect.test.ts
git commit -m "test(frontend): add wsReconnect strategy tests"

git add src/store/connectionStore.ts src/hooks/useWebSocket.ts
git commit -m "feat(frontend): integrate exponential backoff reconnect"

git add src/components/StatusBar.tsx
git commit -m "feat(frontend): show reconnect status in StatusBar"
```

---

*Template: `.agent/tasks/example-feature-ws-reconnect.md`*
