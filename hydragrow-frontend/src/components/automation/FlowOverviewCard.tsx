import type { UserScript } from "../../types/automation";

const DOW_NAMES = ['Chủ nhật', 'Thứ Hai', 'Thứ Ba', 'Thứ Tư', 'Thứ Năm', 'Thứ Sáu', 'Thứ Bảy'];

function cronToVietnamese(expr: string): string | null {
  const parts = expr.trim().split(/\s+/);
  if (parts.length !== 6) return null;
  const [, minuteRaw, hourRaw, , , dowRaw] = parts;
  const minute = minuteRaw === '*' ? 0 : parseInt(minuteRaw, 10);
  const hour = hourRaw === '*' ? 0 : parseInt(hourRaw, 10);
  if (Number.isNaN(minute) || Number.isNaN(hour)) return null;
  const time = `${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}`;
  if (dowRaw === '*' || dowRaw === '?') return `${time} hằng ngày`;
  if (/^\d+$/.test(dowRaw)) return `${time} ${DOW_NAMES[parseInt(dowRaw, 10) % 7]}`;
  if (/^\d+-\d+$/.test(dowRaw)) {
    const [a, b] = dowRaw.split('-').map((d) => DOW_NAMES[parseInt(d, 10) % 7]);
    return `${time} từ ${a} đến ${b}`;
  }
  if (/^[\d,]+$/.test(dowRaw)) {
    const days = dowRaw.split(',').map((d) => DOW_NAMES[parseInt(d, 10) % 7]).join(', ');
    return `${time} ${days}`;
  }
  return `${time} định kỳ`;
}

const DOW_SHORT = ['Chủ nhật', 'Thứ Hai', 'Thứ Ba', 'Thứ Tư', 'Thứ Năm', 'Thứ Sáu', 'Thứ Bảy'];

function lastRunLabel(lastRunAt?: string | null): string {
  if (!lastRunAt) return 'Chưa từng chạy';
  const run = new Date(lastRunAt);
  const now = new Date();
  const startToday = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const startRunDay = new Date(run.getFullYear(), run.getMonth(), run.getDate()).getTime();
  if (startRunDay >= startToday) return 'lần cuối: hôm nay';
  const days = Math.floor((startToday - startRunDay) / 86400000);
  if (days < 7) return `lần cuối: ${DOW_SHORT[run.getDay()]}`;
  return `lần cuối: ${run.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })}`;
}

interface Props {
  script: UserScript;
  onClick: () => void;
  onToggleEnabled?: (e: React.MouseEvent) => void;
}

export function FlowOverviewCard({ script, onClick, onToggleEnabled }: Props) {
  const isConfig = script.kind === "config_override" || (script.ir_json as { kind?: string } | null)?.kind === "config_override" || script.name.toLowerCase().includes("config") || script.name.toLowerCase().includes("ngưỡng ec");
  const kind = isConfig ? "CONFIG" : (script.kind ? script.kind.toUpperCase() : "ALERT");

  const getKindBadgeClass = () => {
    switch (kind) {
      case "ALERT":
        return "bg-amber-100 text-amber-800 border-amber-200";
      case "RECIPE":
      case "RECIPE_OVERRIDE":
        return "bg-pill text-status border-pill";
      case "CONFIG":
      case "CONFIG_OVERRIDE":
        return "bg-config-soft text-config border-config-soft";
      case "ACTION":
      case "ACTION_COMMAND":
        return "bg-sky-100 text-sky-800 border-sky-200";
      default:
        return "bg-pill text-status border-pill";
    }
  };

  const getSummary = () => {
    if (isConfig) {
      const target = script.ir_json?.configOverwrite?.configKey ?? "config.ec_target";
      return `Đọc ${target} · Ghi đè khi điều kiện đúng`;
    }
    const trigger = script.ir_json?.trigger?.type ?? "sensor";
    if (trigger === "cron") {
      const cronExpr = (script.ir_json?.trigger as { cronExpression?: string } | null)?.cronExpression;
      if (cronExpr) {
        const readable = cronToVietnamese(cronExpr);
        if (readable) return `Trigger: ${readable}`;
      }
      return "Trigger: Cron biểu thức lịch định kỳ";
    }
    if (trigger === "webhook") return "Trigger: Webhook nhận dữ liệu bên ngoài";
    if (trigger === "fsm") return "Trigger: FSM giai đoạn canh tác";
    return "Trigger: Cảm biến thời gian thực";
  };

  const triggerKind =
    (script.ir_json?.nodes?.find((n: any) => n.id === "trigger")?.data as any)?.kind ??
    script.ir_json?.trigger?.type;

  const showCronBadge = triggerKind === "cron";
  const showWebhookBadge = triggerKind === "webhook";

  return (
    <div
      onClick={onClick}
      className={`ui-card p-4 rounded-2xl bg-white border border-line hover:border-primary/50 hover:shadow-md transition-all cursor-pointer flex flex-col justify-between h-36 group ${
        !script.enabled ? "opacity-75" : ""
      }`}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <span
            className={`text-[11px] font-bold uppercase tracking-wider px-2 py-0.5 rounded-md border ${getKindBadgeClass()}`}
          >
            {kind === "ACTION_COMMAND" ? "ACTION" : kind === "RECIPE_OVERRIDE" ? "RECIPE" : kind}
          </span>
          {showCronBadge && (
            <span className="text-[10px] font-bold px-1.5 py-0.5 rounded bg-purple-100 text-purple-800 border border-purple-200">
              CRON
            </span>
          )}
          {showWebhookBadge && (
            <span className="text-[10px] font-bold px-1.5 py-0.5 rounded bg-indigo-100 text-indigo-800 border border-indigo-200">
              WEBHOOK
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {script.enabled ? (
            <span
              onClick={onToggleEnabled}
              className="text-xs font-medium px-2.5 py-0.5 rounded-full bg-pill text-status border border-pill"
            >
              Đang bật
            </span>
          ) : (
            <span
              onClick={onToggleEnabled}
              className="text-xs font-medium px-2.5 py-0.5 rounded-full bg-surface-muted text-faint border border-line"
            >
              Đã tắt
            </span>
          )}
        </div>
      </div>

      <div className="my-auto">
        <h4 className="font-semibold text-primary-deep text-base group-hover:text-primary transition-colors line-clamp-1">
          {isConfig ? `★ ${script.name}` : script.name}
        </h4>
        <p className="text-xs text-text-muted mt-1 line-clamp-1">
          {getSummary()}
        </p>
      </div>

      <div className="text-[11px] text-faint flex items-center justify-between pt-1 border-t border-line">
        <span>{lastRunLabel(script.last_run_at)}</span>
        <span className="group-hover:translate-x-0.5 transition-transform text-primary font-medium">Chi tiết &rarr;</span>
      </div>
    </div>
  );
}
