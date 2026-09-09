export interface DosingHistoryRangeRecord {
    created_at: string;
    pump_a_ml: number;
    pump_b_ml: number;
    ph_up_ml: number;
    ph_down_ml: number;
}

type PumpField = 'pump_a_ml' | 'pump_b_ml' | 'ph_up_ml' | 'ph_down_ml';
const PUMP_FIELDS: PumpField[] = ['pump_a_ml', 'pump_b_ml', 'ph_up_ml', 'ph_down_ml'];

const recordTotal = (r: DosingHistoryRangeRecord): number =>
    r.pump_a_ml + r.pump_b_ml + r.ph_up_ml + r.ph_down_ml;

export const totalMlToday = (records: DosingHistoryRangeRecord[]): number => {
    const startOfToday = new Date();
    startOfToday.setHours(0, 0, 0, 0);
    return records
        .filter((r) => new Date(r.created_at) >= startOfToday)
        .reduce((sum, r) => sum + recordTotal(r), 0);
};

export const hourlyBuckets = (records: DosingHistoryRangeRecord[]): number[] => {
    const buckets = new Array(24).fill(0);
    for (const r of records) {
        const hour = new Date(r.created_at).getHours();
        buckets[hour] += recordTotal(r);
    }
    return buckets;
};

export const sevenDayAverage = (
    records: DosingHistoryRangeRecord[],
): { averageMlPerDay: number; changePercentVsPrevious7Days: number } => {
    const now = Date.now();
    const sevenDaysAgo = now - 7 * 86400000;
    const fourteenDaysAgo = now - 14 * 86400000;

    const last7 = records.filter((r) => new Date(r.created_at).getTime() >= sevenDaysAgo);
    const prev7 = records.filter((r) => {
        const t = new Date(r.created_at).getTime();
        return t >= fourteenDaysAgo && t < sevenDaysAgo;
    });

    const last7Total = last7.reduce((sum, r) => sum + recordTotal(r), 0);
    const prev7Total = prev7.reduce((sum, r) => sum + recordTotal(r), 0);
    const averageMlPerDay = last7Total / 7;
    const changePercentVsPrevious7Days =
        prev7Total > 0 ? ((last7Total - prev7Total) / prev7Total) * 100 : 0;

    return { averageMlPerDay, changePercentVsPrevious7Days };
};

export interface DosingAnomaly {
    pump: PumpField;
    ratio: number;
}

export const detectAnomalies = (records: DosingHistoryRangeRecord[]): DosingAnomaly[] => {
    const now = Date.now();
    const twoHoursAgo = now - 2 * 3600000;
    const sevenDaysAgo = now - 7 * 86400000;

    const anomalies: DosingAnomaly[] = [];
    for (const pump of PUMP_FIELDS) {
        const recentCycles = records.filter(
            (r) => r[pump] > 0 && new Date(r.created_at).getTime() >= twoHoursAgo,
        );
        if (recentCycles.length === 0) continue;

        const baselineCycles = records.filter(
            (r) => r[pump] > 0 && new Date(r.created_at).getTime() >= sevenDaysAgo,
        );
        if (baselineCycles.length === 0) continue;

        const baselineAvg =
            baselineCycles.reduce((sum, r) => sum + r[pump], 0) / baselineCycles.length;
        const recentAvg =
            recentCycles.reduce((sum, r) => sum + r[pump], 0) / recentCycles.length;

        if (baselineAvg > 0 && recentAvg / baselineAvg >= 3) {
            anomalies.push({ pump, ratio: recentAvg / baselineAvg });
        }
    }
    return anomalies;
};
