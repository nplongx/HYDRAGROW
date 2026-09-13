import { DosingHourlyChart } from './DosingHourlyChart';
import type { PumpField } from '../../lib/dosing/dosingAggregates';

interface DosingTotalCardProps {
    totalMlToday: number;
    bucketsByPump: Record<PumpField, number[]>;
    averageMlPerDay: number;
    changePercent: number;
}

export const DosingTotalCard = ({
    totalMlToday,
    bucketsByPump,
    averageMlPerDay,
    changePercent,
}: DosingTotalCardProps) => {
    return (
        <div className="bg-white border border-line rounded-2xl p-4 space-y-3 shadow-sm">
            <p className="text-[10px] font-bold uppercase tracking-wider text-faint">Tổng lượng châm hôm nay</p>
            <p className="text-3xl font-black text-primary-deep">{totalMlToday.toFixed(0)} ml</p>
            <DosingHourlyChart bucketsByPump={bucketsByPump} />
            <p className="text-xs text-text-muted pt-1">
                Trung bình 7 ngày: <b>{averageMlPerDay.toFixed(0)} ml/ngày</b>
                {changePercent !== 0 && (
                    <span className={changePercent > 0 ? 'text-error font-bold' : 'text-status font-bold'}>
                        {' '}({changePercent > 0 ? '+' : ''}{changePercent.toFixed(0)}% so với tuần trước)
                    </span>
                )}
            </p>
        </div>
    );
};
