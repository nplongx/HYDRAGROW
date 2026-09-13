import { useRef } from 'react';

/** Số mẫu tối đa giữ trong bộ nhớ phiên — 15s/mẫu * 40 ≈ 10 phút xu hướng gần nhất. */
export const MAX_HEALTH_SAMPLES = 40;

export function pushHealthSample(buffer: number[], value: number | null | undefined): number[] {
  if (value === null || value === undefined) return buffer;
  const next = [...buffer, value];
  return next.length > MAX_HEALTH_SAMPLES ? next.slice(next.length - MAX_HEALTH_SAMPLES) : next;
}

/**
 * Ring-buffer xu hướng trong bộ nhớ phiên cho 1 chỉ số sức khoẻ phần cứng.
 * Không có endpoint lịch sử ở backend cho các chỉ số này — reset khi tải
 * lại trang là đánh đổi chấp nhận được; lịch sử dài hạn xem qua Grafana
 * (đã có, xem Analytics.tsx phần grafanaUrl).
 */
export function useHealthHistory() {
  const ref = useRef<Record<string, number[]>>({});

  const record = (key: string, value: number | null | undefined) => {
    ref.current[key] = pushHealthSample(ref.current[key] ?? [], value);
    return ref.current[key];
  };

  return { record };
}
