import type { UserScript } from "../../types/automation";

interface Props {
  script: UserScript;
  onClick: () => void;
  onToggleEnabled?: (e: React.MouseEvent) => void;
}

export function FlowOverviewCard({ script, onClick, onToggleEnabled }: Props) {
  const isConfig = script.kind === "config_override" || script.ir_json?.kind === "config_override" || script.name.toLowerCase().includes("config") || script.name.toLowerCase().includes("ngưỡng ec");
  const kind = isConfig ? "CONFIG" : (script.kind ? script.kind.toUpperCase() : "ALERT");

  const getKindBadgeClass = () => {
    switch (kind) {
      case "ALERT":
        return "bg-amber-100 text-amber-800 border-amber-200";
      case "RECIPE":
      case "RECIPE_OVERRIDE":
        return "bg-emerald-100 text-emerald-800 border-emerald-200";
      case "CONFIG":
      case "CONFIG_OVERRIDE":
        return "bg-indigo-100 text-indigo-800 border-indigo-200";
      case "ACTION":
      case "ACTION_COMMAND":
        return "bg-sky-100 text-sky-800 border-sky-200";
      default:
        return "bg-emerald-100 text-emerald-800 border-emerald-200";
    }
  };

  const getSummary = () => {
    if (isConfig) {
      const target = script.ir_json?.configOverwrite?.configKey ?? "config.ec_target";
      return `Đọc ${target} · Ghi đè khi điều kiện đúng`;
    }
    const trigger = script.ir_json?.trigger?.type ?? "sensor";
    if (trigger === "cron") return "Trigger: Cron biểu thức lịch định kỳ";
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
      className={`ui-card p-4 rounded-2xl bg-white border border-emerald-100/80 hover:border-emerald-300 hover:shadow-md transition-all cursor-pointer flex flex-col justify-between h-36 group ${
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
              className="text-xs font-medium px-2.5 py-0.5 rounded-full bg-emerald-50 text-emerald-700 border border-emerald-200/60"
            >
              Đang bật
            </span>
          ) : (
            <span
              onClick={onToggleEnabled}
              className="text-xs font-medium px-2.5 py-0.5 rounded-full bg-slate-100 text-slate-600 border border-slate-200"
            >
              Đã tắt
            </span>
          )}
        </div>
      </div>

      <div className="my-auto">
        <h4 className="font-semibold text-emerald-950 text-base group-hover:text-emerald-700 transition-colors line-clamp-1">
          {isConfig ? `★ ${script.name}` : script.name}
        </h4>
        <p className="text-xs text-emerald-800/70 mt-1 line-clamp-1">
          {getSummary()}
        </p>
      </div>

      <div className="text-[11px] text-emerald-800/50 flex items-center justify-between pt-1 border-t border-emerald-50">
        <span>Cập nhật gần đây</span>
        <span className="group-hover:translate-x-0.5 transition-transform text-emerald-600 font-medium">Chi tiết &rarr;</span>
      </div>
    </div>
  );
}
