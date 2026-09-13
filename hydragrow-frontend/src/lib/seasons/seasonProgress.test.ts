import { describe, it, expect } from 'vitest';
import { totalPlannedDays, elapsedDays, expectedStageEndDay, delayDays, stageChecklist, remainingDays } from './seasonProgress';
import type { CropStage } from '../../types/models';

const stage = (durationDays: number): CropStage => ({
    name: 'x',
    duration_sec: durationDays * 86400,
    ec_target: 1.8,
    ec_tolerance: 0.2,
    ph_target: 6,
    ph_tolerance: 0.3,
    nutrient_a_ratio: 1,
    nutrient_b_ratio: 1,
    water_level_target: 10,
    misting_on_duration_ms: 1000,
    misting_off_duration_ms: 1000,
});

describe('totalPlannedDays', () => {
    it('cộng dồn duration_sec của tất cả giai đoạn ra số ngày', () => {
        expect(totalPlannedDays([stage(10), stage(15), stage(10)])).toBe(35);
    });
});

describe('elapsedDays', () => {
    it('tính số ngày đã trôi qua từ start_time tới hiện tại', () => {
        const start = new Date(Date.now() - 5 * 86400000).toISOString();
        expect(elapsedDays(start)).toBeCloseTo(5, 0);
    });
});

describe('expectedStageEndDay', () => {
    it('trả tổng số ngày dự kiến kết thúc giai đoạn hiện tại (tính từ đầu mùa vụ)', () => {
        const stages = [stage(10), stage(15), stage(10)];
        expect(expectedStageEndDay(stages, 0)).toBe(10);
        expect(expectedStageEndDay(stages, 1)).toBe(25);
    });
});

describe('delayDays', () => {
    it('trả 0 khi chưa quá hạn giai đoạn hiện tại', () => {
        const stages = [stage(10), stage(15), stage(10)];
        expect(delayDays(stages, 1, 20)).toBe(0);
    });

    it('trả số ngày chậm khi đã qua hạn dự kiến của giai đoạn hiện tại', () => {
        const stages = [stage(10), stage(15), stage(10)];
        expect(delayDays(stages, 1, 28)).toBe(3);
    });
});

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

