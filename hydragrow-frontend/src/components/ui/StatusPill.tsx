type StatusPillKind = 'sending' | 'accepted' | 'error';

const STYLES: Record<StatusPillKind, string> = {
  sending: 'bg-amber-50 text-amber-700 border-amber-200',
  accepted: 'bg-emerald-50 text-emerald-700 border-emerald-200',
  error: 'bg-red-50 text-red-700 border-red-200',
};

const LABELS: Record<StatusPillKind, string> = {
  sending: 'Đang gửi…',
  accepted: '✓ Xác nhận',
  error: '⚠ Lỗi phản hồi',
};

const kindOf = (commandStatus?: string): StatusPillKind | null => {
  if (!commandStatus) return null;
  if (commandStatus === 'sending') return 'sending';
  if (commandStatus === 'accepted') return 'accepted';
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
      className={`inline-flex items-center px-2 py-0.5 rounded-full border text-[10px] font-bold ${STYLES[kind]}`}
    >
      {LABELS[kind]}
    </span>
  );
};
