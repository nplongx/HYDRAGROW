import { describe, expect, it } from 'vitest';
import { buildLogRows, extractCycleId, filterEventsBySearch } from './eventGrouping';
import type { SystemEvent } from '../../components/logs/EventLogCard';

const baseEvent = (overrides: Partial<SystemEvent>): SystemEvent => ({
  id: overrides.id ?? 1,
  device_id: 'device-1',
  level: 'info',
  category: 'system',
  title: 'Giám sát',
  message: 'Monitoring',
  timestamp: 1000,
  ...overrides,
});

describe('extractCycleId', () => {
  it('reads top-level metadata.cycle_id', () => {
    expect(extractCycleId(baseEvent({ metadata: { cycle_id: 'cyc-1' } }))).toBe('cyc-1');
  });

  it('falls back to metadata.dosing_data.cycle_id', () => {
    expect(
      extractCycleId(baseEvent({ metadata: { dosing_data: { cycle_id: 'cyc-2' } } }))
    ).toBe('cyc-2');
  });

  it('returns null when no cycle_id present', () => {
    expect(extractCycleId(baseEvent({ metadata: { source: 'fsm' } }))).toBeNull();
  });
});

describe('filterEventsBySearch', () => {
  const events = [
    baseEvent({ id: 1, title: 'Châm EC', message: 'Bơm A chạy 5s' }),
    baseEvent({ id: 2, title: 'Mất kết nối', message: 'Wifi rớt' }),
  ];

  it('returns all events on empty query', () => {
    expect(filterEventsBySearch(events, '')).toHaveLength(2);
  });

  it('matches title case-insensitively', () => {
    expect(filterEventsBySearch(events, 'châm')).toEqual([events[0]]);
  });

  it('matches message content', () => {
    expect(filterEventsBySearch(events, 'wifi')).toEqual([events[1]]);
  });
});

describe('buildLogRows — chế độ important', () => {
  it('gộp các sự kiện kỹ thuật info liên tiếp cùng title/category thành 1 dòng merged', () => {
    const events = [
      baseEvent({ id: 1, category: 'system', level: 'info', title: 'Giám sát', timestamp: 1000 }),
      baseEvent({ id: 2, category: 'system', level: 'info', title: 'Giám sát', timestamp: 2000 }),
      baseEvent({ id: 3, category: 'system', level: 'info', title: 'Giám sát', timestamp: 3000 }),
    ];
    const rows = buildLogRows(events, 'important');
    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({ type: 'merged', count: 3, title: 'Giám sát', category: 'system' });
  });

  it('không gộp cảnh báo/nghiêm trọng dù cùng category', () => {
    const events = [
      baseEvent({ id: 1, category: 'sensor', level: 'warning', title: 'Nhiễu cảm biến', timestamp: 1000 }),
      baseEvent({ id: 2, category: 'sensor', level: 'warning', title: 'Nhiễu cảm biến', timestamp: 2000 }),
    ];
    const rows = buildLogRows(events, 'important');
    expect(rows).toHaveLength(2);
    expect(rows.every((r) => r.type === 'event')).toBe(true);
  });

  it('gom các event cùng cycle_id thành 1 dòng cycle, giữ đúng vị trí xuất hiện đầu tiên', () => {
    const events = [
      baseEvent({ id: 1, category: 'dosing', title: 'Bắt đầu châm', metadata: { cycle_id: 'cyc-9' }, timestamp: 1000 }),
      baseEvent({ id: 2, category: 'system', level: 'info', title: 'Giám sát', timestamp: 1500 }),
      baseEvent({ id: 3, category: 'dosing', title: 'Hoàn tất châm', metadata: { cycle_id: 'cyc-9' }, timestamp: 2000 }),
    ];
    const rows = buildLogRows(events, 'important');
    expect(rows[0]).toMatchObject({ type: 'cycle', cycleId: 'cyc-9' });
    expect((rows[0] as { type: string; cycleId: string; events: SystemEvent[] }).events).toHaveLength(2);
    expect(rows[1]).toMatchObject({ type: 'event' });
  });
});

describe('buildLogRows — chế độ all_technical', () => {
  it('không gộp dòng kỹ thuật, giữ nguyên từng event riêng lẻ', () => {
    const events = [
      baseEvent({ id: 1, category: 'sensor', level: 'info', title: 'Đọc cảm biến', timestamp: 1000 }),
      baseEvent({ id: 2, category: 'sensor', level: 'info', title: 'Đọc cảm biến', timestamp: 2000 }),
    ];
    const rows = buildLogRows(events, 'all_technical');
    expect(rows).toHaveLength(2);
    expect(rows.every((r) => r.type === 'event')).toBe(true);
  });

  it('vẫn gom chu trình theo cycle_id kể cả ở chế độ all_technical', () => {
    const events = [
      baseEvent({ id: 1, category: 'water', title: 'Cấp nước', metadata: { cycle_id: 'cyc-5' }, timestamp: 1000 }),
      baseEvent({ id: 2, category: 'water', title: 'Xả nước', metadata: { cycle_id: 'cyc-5' }, timestamp: 2000 }),
    ];
    const rows = buildLogRows(events, 'all_technical');
    expect(rows).toHaveLength(1);
    expect(rows[0].type).toBe('cycle');
  });
});
