import type { SystemEvent } from '../../components/logs/EventLogCard';

export type LogViewMode = 'important' | 'all_technical';

export type LogRow =
  | { type: 'event'; event: SystemEvent }
  | { type: 'merged'; title: string; category: string; count: number; events: SystemEvent[]; latestTimestamp: number }
  | { type: 'cycle'; cycleId: string; events: SystemEvent[] };

/** Category coi là "nhiễu kỹ thuật" khi ở level info: FSM transition (system),
 * calibration debug (calibration), sensor reading (sensor). Không bao giờ gộp
 * warning/critical/success dù cùng category — an toàn không mất cảnh báo. */
const TECHNICAL_NOISE_CATEGORIES = new Set(['system', 'sensor', 'calibration']);

/** Đọc cycle_id từ metadata — khớp cả 2 đường mà backend get_events_by_cycle_id
 * đang tìm: metadata.cycle_id (top-level) và metadata.dosing_data.cycle_id. */
export function extractCycleId(event: SystemEvent): string | null {
  const meta = event.metadata;
  if (!meta) return null;
  if (typeof meta.cycle_id === 'string' && meta.cycle_id.length > 0) return meta.cycle_id;
  const dosingData = meta.dosing_data;
  if (dosingData && typeof dosingData === 'object' && typeof dosingData.cycle_id === 'string') {
    return dosingData.cycle_id;
  }
  return null;
}

export function filterEventsBySearch(events: SystemEvent[], query: string): SystemEvent[] {
  const q = query.trim().toLowerCase();
  if (!q) return events;
  return events.filter((ev) => {
    const haystack = [ev.title, ev.message, ev.category, ev.reason].filter(Boolean).join(' ').toLowerCase();
    return haystack.includes(q);
  });
}

/** Gom mọi event có chung cycle_id thành 1 dòng 'cycle', đặt tại vị trí xuất hiện
 * đầu tiên của cycle_id đó trong mảng gốc (giữ thứ tự thời gian tổng thể). */
function groupEventsByCycle(events: SystemEvent[]): LogRow[] {
  const cycleEvents = new Map<string, SystemEvent[]>();

  events.forEach((ev) => {
    const cycleId = extractCycleId(ev);
    if (!cycleId) return;
    if (!cycleEvents.has(cycleId)) cycleEvents.set(cycleId, []);
    cycleEvents.get(cycleId)!.push(ev);
  });

  const rows: LogRow[] = [];
  const seenCycles = new Set<string>();

  events.forEach((ev) => {
    const cycleId = extractCycleId(ev);
    if (!cycleId) {
      rows.push({ type: 'event', event: ev });
      return;
    }
    if (seenCycles.has(cycleId)) return; // đã emit ở vị trí xuất hiện đầu tiên
    seenCycles.add(cycleId);
    rows.push({ type: 'cycle', cycleId, events: cycleEvents.get(cycleId)! });
  });

  return rows;
}

/** Gộp các dòng 'event' liên tiếp cùng category kỹ thuật + level info + cùng title
 * thành 1 dòng 'merged'. Không đụng tới dòng 'cycle' (luôn giữ nguyên). */
function mergeTechnicalNoise(rows: LogRow[]): LogRow[] {
  const result: LogRow[] = [];
  let run: SystemEvent[] = [];

  const flushRun = () => {
    if (run.length === 0) return;
    if (run.length === 1) {
      result.push({ type: 'event', event: run[0] });
    } else {
      result.push({
        type: 'merged',
        title: run[0].title,
        category: run[0].category,
        count: run.length,
        events: [...run],
        latestTimestamp: run[run.length - 1].timestamp,
      });
    }
    run = [];
  };

  for (const row of rows) {
    if (row.type !== 'event') {
      flushRun();
      result.push(row);
      continue;
    }
    const ev = row.event;
    const isNoise = TECHNICAL_NOISE_CATEGORIES.has(ev.category) && ev.level === 'info';
    if (!isNoise) {
      flushRun();
      result.push(row);
      continue;
    }
    const last = run[run.length - 1];
    if (last && last.category === ev.category && last.title === ev.title) {
      run.push(ev);
    } else {
      flushRun();
      run.push(ev);
    }
  }
  flushRun();

  return result;
}

/** Điểm vào chính: sắp xếp thành các dòng hiển thị theo chế độ xem.
 * - Gom chu trình theo cycle_id luôn áp dụng, ở cả 2 chế độ (Màn 03).
 * - Gộp nhiễu kỹ thuật (heartbeat/FSM/sensor/calibration info) chỉ áp dụng
 * ở chế độ 'important' (Màn 01). Ở 'all_technical', mọi event hiện riêng (Màn 02). */
export function buildLogRows(events: SystemEvent[], mode: LogViewMode): LogRow[] {
  const cycled = groupEventsByCycle(events);
  if (mode === 'all_technical') return cycled;
  return mergeTechnicalNoise(cycled);
}
