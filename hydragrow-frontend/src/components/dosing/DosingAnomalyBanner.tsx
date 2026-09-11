import { Banner } from '../ui/Banner';
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
    <Banner tone="danger" title={`Bất thường: ${PUMP_LABEL[first.pump] ?? first.pump} đã châm gấp ~${first.ratio.toFixed(0)} lần mức trung bình trong 2 giờ qua`} />
  );
};