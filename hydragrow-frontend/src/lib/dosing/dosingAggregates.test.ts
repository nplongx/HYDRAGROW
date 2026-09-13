import { describe, it, expect } from 'vitest';
import { totalMlToday, hourlyBuckets, sevenDayAverage, detectAnomalies, hourlyBucketsByPump } from './dosingAggregates';
import type { DosingHistoryRangeRecord } from './dosingAggregates';

const record = (hoursAgo: number, overrides: Partial<DosingHistoryRangeRecord> = {}): DosingHistoryRangeRecord => ({
    created_at: new Date(Date.now() - hoursAgo * 3600000).toISOString(),
    pump_a_ml: 0,
    pump_b_ml: 0,
    ph_up_ml: 0,
    ph_down_ml: 0,
    ...overrides,
});

describe('totalMlToday', () => {
    it('cộng tổng ml của các record trong ngày hôm nay', () => {
        const records = [record(1, { pump_a_ml: 10 }), record(2, { ph_up_ml: 5 })];
        expect(totalMlToday(records)).toBe(15);
    });
});

describe('hourlyBuckets', () => {
    it('trả mảng 24 phần tử', () => {
        expect(hourlyBuckets([record(1, { pump_a_ml: 10 })])).toHaveLength(24);
    });
});

describe('sevenDayAverage', () => {
    it('tính trung bình ml/ngày trong 7 ngày gần nhất', () => {
        const records = [1, 2, 3, 4, 5, 6, 7].map((d) =>
            record(d * 24, { pump_a_ml: 2 }),
        );
        const result = sevenDayAverage(records);
        expect(result.averageMlPerDay).toBeCloseTo(2, 0);
    });
});

describe('detectAnomalies', () => {
    it('phát hiện bơm châm gấp >= 3 lần trung bình 7 ngày trong 2 giờ qua', () => {
        const normalCycles = Array.from({ length: 14 }, (_, i) => record(24 + i * 12, { ph_down_ml: 1 }));
        const spike = record(1, { ph_down_ml: 10 });
        const anomalies = detectAnomalies([...normalCycles, spike]);
        expect(anomalies.some((a) => a.pump === 'ph_down_ml')).toBe(true);
    });

    it('không báo bất thường khi lượng châm ổn định', () => {
        const normalCycles = Array.from({ length: 14 }, (_, i) => record(1 + i * 12, { pump_a_ml: 5 }));
        expect(detectAnomalies(normalCycles)).toHaveLength(0);
    });
});

describe('hourlyBucketsByPump', () => {
    it('trả về 4 mảng 24 phần tử, tách riêng theo từng field bơm', () => {
        const records = [
            { created_at: new Date(new Date().setHours(9, 0, 0, 0)).toISOString(), pump_a_ml: 5, pump_b_ml: 0, ph_up_ml: 0, ph_down_ml: 0 },
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
