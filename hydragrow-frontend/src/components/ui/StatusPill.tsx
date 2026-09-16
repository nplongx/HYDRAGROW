type StatusPillKind = 'sending' | 'acknowledged' | 'confirmed' | 'error';

const STYLES: Record<StatusPillKind, string> = {
  sending: 'bg-warning-bg text-warn-deep',
  acknowledged: 'bg-pill text-status',
  confirmed: 'bg-pill text-status',
  error: 'bg-danger-bg text-error',
};

const LABELS: Record<StatusPillKind, string> = {
  sending: 'Đang gửi…',
  acknowledged: 'Đã nhận lệnh',
  confirmed: '✓ Đã xác nhận',
  error: '⚠ Lỗi phản hồi',
};

const kindOf = (commandStatus?: string): StatusPillKind | null => {
  if (!commandStatus) return null;
  if (commandStatus === 'REQUESTED' || commandStatus === 'SENT') return 'sending';
  if (commandStatus === 'ACKNOWLEDGED') return 'acknowledged';
  if (commandStatus === 'CONFIRMED') return 'confirmed';
  return 'error';
};

interface StatusPillProps {
  commandStatus?: string;
}

export const StatusPill = ({ commandStatus }: StatusPillProps) => {
  const kind = kindOf(commandStatus);
  if (!kind) return null;
  return (
    <span
      title={commandStatus}
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-[10px] font-semibold ${STYLES[kind]}`}
    >
      {LABELS[kind]}
    </span>
  );
};
