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
        <div className="bg-white border border-emerald-100 rounded-2xl p-4 space-y-3 shadow-sm">
            <p className="text-[10px] font-bold uppercase tracking-wider text-emerald-700/70">Tổng lượng châm hôm nay</p>
            <p className="text-3xl font-black text-emerald-950">{totalMlToday.toFixed(0)} ml</p>
            <div className="flex items-end gap-1 h-20 pt-2">
                {hourlyValues.map((v, hour) => (
                    <div
                        key={hour}
                        title={`Giờ ${hour}: ${v.toFixed(1)} ml`}
                        className="flex-1 bg-emerald-200 hover:bg-emerald-400 transition-colors rounded-t"
                        style={{ height: `${Math.max(4, (v / max) * 100)}%` }}
                    />
                ))}
            </div>
            <p className="text-xs text-emerald-800/75 pt-1">
                Trung bình 7 ngày: <b>{averageMlPerDay.toFixed(0)} ml/ngày</b>
                {changePercent !== 0 && (
                    <span className={changePercent > 0 ? 'text-red-600 font-bold' : 'text-emerald-600 font-bold'}>
                        {' '}({changePercent > 0 ? '+' : ''}{changePercent.toFixed(0)}% so với tuần trước)
                    </span>
                )}
            </p>
        </div>
    );
};
