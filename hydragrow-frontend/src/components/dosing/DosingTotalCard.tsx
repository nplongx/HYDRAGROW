interface DosingTotalCardProps {
    totalMlToday: number;
    hourlyValues: number[];
    averageMlPerDay: number;
    changePercent: number;
}

export const DosingTotalCard = ({
    totalMlToday,
    hourlyValues,
    averageMlPerDay,
    changePercent,
}: DosingTotalCardProps) => {
    const max = Math.max(1, ...hourlyValues);
    return (
        <div className="bg-white border border-line rounded-2xl p-4 space-y-3 shadow-sm">
            <p className="text-[10px] font-bold uppercase tracking-wider text-faint">Tổng lượng châm hôm nay</p>
            <p className="text-3xl font-black text-primary-deep">{totalMlToday.toFixed(0)} ml</p>
            <div className="flex items-end gap-1 h-20 pt-2">
                {hourlyValues.map((v, hour) => (
                    <div
                        key={hour}
                        title={`Giờ ${hour}: ${v.toFixed(1)} ml`}
                        className="flex-1 bg-primary hover:bg-primary-deep transition-colors rounded-t"
                        style={{ height: `${Math.max(4, (v / max) * 100)}%` }}
                    />
                ))}
            </div>
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
