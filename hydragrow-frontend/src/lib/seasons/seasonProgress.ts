import type { CropStage } from '../../types/models';

const SEC_PER_DAY = 86400;

export const totalPlannedDays = (stages: CropStage[]): number =>
    stages.reduce((sum, s) => sum + s.duration_sec, 0) / SEC_PER_DAY;

export const elapsedDays = (startTime: string): number =>
    (Date.now() - new Date(startTime).getTime()) / (SEC_PER_DAY * 1000);

export const expectedStageEndDay = (stages: CropStage[], currentStageIndex: number): number =>
    stages.slice(0, currentStageIndex + 1).reduce((sum, s) => sum + s.duration_sec, 0) / SEC_PER_DAY;

export const delayDays = (
    stages: CropStage[],
    currentStageIndex: number,
    elapsed: number,
): number => {
    const expectedEnd = expectedStageEndDay(stages, currentStageIndex);
    return elapsed > expectedEnd ? elapsed - expectedEnd : 0;
};

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

