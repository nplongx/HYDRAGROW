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
