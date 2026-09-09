import { AlertTriangle } from 'lucide-react';
import type { DosingAnomaly } from '../../lib/dosing/dosingAggregates';

const PUMP_LABEL: Record<string, string> = {
    pump_a_ml: 'Bơm phân A',
    pump_b_ml: 'Bơm phân B',
    ph_up_ml: 'Bơm pH Up',
    ph_down_ml: 'Bơm pH Down',
};

export const DosingAnomalyBanner = ({ anomalies }: { anomalies: DosingAnomaly[] }) => {
    if (anomalies.length === 0) return null;
    const first = anomalies[0];
    return (
        <div className="bg-red-50 border border-red-200 rounded-2xl p-4 flex items-start gap-3 text-red-900 shadow-sm">
            <AlertTriangle className="text-red-600 shrink-0 mt-0.5" size={20} />
            <div>
                <span className="text-[10px] font-black uppercase tracking-wider bg-red-100 text-red-700 px-2 py-0.5 rounded-full mr-2">
                    Bất thường
                </span>
                <p className="text-sm font-semibold inline">
                    {PUMP_LABEL[first.pump] || first.pump} đã châm gấp ~{first.ratio.toFixed(0)} lần mức trung bình trong 2 giờ qua
                </p>
            </div>
        </div>
    );
};
