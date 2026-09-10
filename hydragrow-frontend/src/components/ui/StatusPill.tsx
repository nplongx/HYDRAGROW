type StatusPillKind = 'sending' | 'accepted' | 'error';

const STYLES: Record<StatusPillKind, string> = {
  sending: 'bg-[#FFFBEB] text-warn-deep',
  accepted: 'bg-pill text-status',
  error: 'bg-[#FEE2E2] text-error',
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
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-[10px] font-semibold ${STYLES[kind]}`}
    >
      {LABELS[kind]}
    </span>
  );
};
