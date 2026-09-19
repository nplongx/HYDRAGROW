import { Handle, Position } from "@xyflow/react";
import {
  Activity,
  Beaker,
  Bell,
  Link2,
  Zap,
  Clock,
  Filter,
  Calendar,
  Webhook,
  Database,
} from "lucide-react";

interface NodeProps {
  data: any;
  selected?: boolean;
}

export function TriggerNode({ data, selected }: NodeProps) {
  let Icon = Activity;
  let badge = "TRIGGER · SENSOR";
  let title = data.field ? `${data.field} (thời gian thực)` : "Cảm biến (thời gian thực)";
  let subtitle = data.intervalSec ? `Đọc mỗi ${data.intervalSec}s từ cảm biến` : "Đọc mỗi 30s từ cảm biến";
  let borderClass = "border-info bg-info-bg/40 text-info";
  let badgeClass = "bg-info-bg text-info border-info";

  if (data.kind === "fsm") {
    Icon = Zap;
    badge = "TRIGGER · FSM";
    title = data.state ? `Giai đoạn: ${data.state}` : "Giai đoạn canh tác (FSM)";
    subtitle = "Theo dõi chuyển đổi trạng thái FSM";
    borderClass = "border-warning bg-warning-bg/40 text-warn-deep";
    badgeClass = "bg-warning-bg text-warning border-warning";
  } else if (data.kind === "cron") {
    Icon = Calendar;
    badge = "TRIGGER · CRON";
    title = data.expression ? `${data.expression}` : "07:00 mỗi ngày";
    subtitle = data.timezone || "Asia/Ho_Chi_Minh";
    borderClass = "border-config bg-config-soft text-primary-deep";
    badgeClass = "bg-config-soft text-config border-info";
  } else if (data.kind === "webhook") {
    Icon = Webhook;
    badge = "TRIGGER · WEBHOOK";
    title = "Nhận dữ liệu bên ngoài";
    subtitle = data.mode === "direct" ? "Xử lý trực tiếp" : "Kích hoạt Flow";
    borderClass = "border-line bg-config-soft/40 text-primary-deep";
    badgeClass = "bg-config-soft text-config border-info";
  }

  return (
    <div
      className={`min-w-[190px] rounded-2xl border-2 px-3.5 py-2.5 shadow-sm bg-white transition-all cursor-pointer ${borderClass} ${
        selected ? "ring-2 ring-primary ring-offset-2 shadow-md" : ""
      }`}
    >
      <Handle type="target" position={Position.Left} className="opacity-0" />
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-1.5">
          <span className={`text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded border ${badgeClass}`}>
            {badge}
          </span>
          <Icon className="w-3.5 h-3.5 opacity-70" />
        </div>
        <div className="font-bold text-xs text-text mt-0.5 leading-tight">{title}</div>
        <div className="text-[10px] text-text-muted leading-tight truncate max-w-[180px]">{subtitle}</div>
      </div>
      <Handle type="source" position={Position.Right} className="!w-2 !h-2 !bg-primary" />
    </div>
  );
}

export function ConditionNode({ data, selected }: NodeProps) {
  const isGroup = data.type === "condition_group" || Array.isArray(data.conditions);
  const isTimeWindow = data.type === "time-window";
  
  let badge = "CONDITION";
  let title = "Điều kiện an toàn";
  let subtitle = "Kiểm tra biểu thức";

  if (isGroup) {
    const op = data.op?.toUpperCase() ?? "AND";
    badge = `CONDITION · NHÓM [${op}]`;
    title = op === "AND" ? "Tất cả đều đúng" : "Một trong số điều kiện đúng";
    subtitle = data.conditions?.length ? `${data.conditions.length} điều kiện con` : "Chưa có điều kiện con";
  } else if (isTimeWindow) {
    badge = "CONDITION · THỜI GIAN";
    title = data.field ? `${data.field} (${data.mode || "mean"})` : "Khung giờ điều kiện";
    subtitle = data.windowMin ? `Trong ${data.windowMin} phút` : "Cửa sổ thời gian";
  } else if (data.field) {
    title = `${data.field} ${data.op || ">"} ${data.value ?? ""}`;
    subtitle = "So sánh cảm biến tức thời";
  }

  return (
    <div
      className={`min-w-[190px] rounded-2xl border-2 border-warning bg-white px-3.5 py-2.5 shadow-sm transition-all cursor-pointer ${
        selected ? "ring-2 ring-warning ring-offset-2 shadow-md" : ""
      }`}
    >
      <Handle type="target" position={Position.Left} className="!w-2 !h-2 !bg-warning" />
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-1.5">
          <span className="text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded border bg-warning-bg text-warn-deep border-warning">
            {badge}
          </span>
          <Filter className="w-3.5 h-3.5 text-warning" />
        </div>
        <div className="font-bold text-xs text-text mt-0.5 leading-tight">{title}</div>
        <div className="text-[10px] text-text-muted leading-tight truncate max-w-[180px]">{subtitle}</div>
      </div>
      <Handle type="source" position={Position.Right} className="!w-2 !h-2 !bg-warning" />
    </div>
  );
}

export function ActionNode({ data, selected }: NodeProps) {
  let Icon = Zap;
  let badge = "ACTION";
  let title = "Hành động thực thi";
  let subtitle = "Kích hoạt thiết bị";
  let borderClass = "border-line";
  let badgeClass = "bg-pill text-text-muted border-line";

  if (data.type === "alert") {
    Icon = Bell;
    badge = "ACTION · ALERT";
    title = data.message ? `Cảnh báo: ${data.message}` : "Gửi thông báo & Alert";
    subtitle = data.level ? `Mức độ: ${data.level}` : "Gửi FCM / App / Email";
    borderClass = "border-fault";
    badgeClass = "bg-danger-bg text-error border-fault";
  } else if (data.type === "control") {
    Icon = Beaker;
    badge = "ACTION · DOSE/WATER";
    title = data.pump ? `Bơm ${data.pump}` : "Định lượng dinh dưỡng / Bơm";
    subtitle = data.duration ? `Thời gian: ${data.duration}s` : "Bơm A + B hoặc tưới";
    borderClass = "border-info";
    badgeClass = "bg-info-bg text-info border-info";
  } else if (data.type === "delay") {
    Icon = Clock;
    badge = "ACTION · DELAY";
    title = `Chờ ${data.duration || 10}s`;
    subtitle = "Tạm dừng trước bước kế tiếp";
    borderClass = "border-warning";
    badgeClass = "bg-warning-bg text-warning border-warning";
  } else if (data.type === "chain") {
    Icon = Link2;
    badge = "ACTION · CHAIN";
    title = "Chạy tiếp Flow khác";
    subtitle = data.targetFlowName || "Kích hoạt Flow liên kết";
    borderClass = "border-line";
    badgeClass = "bg-config-soft text-config border-info";
  }

  return (
    <div
      className={`min-w-[190px] rounded-2xl border-2 ${borderClass} bg-white px-3.5 py-2.5 shadow-sm transition-all cursor-pointer ${
        selected ? "ring-2 ring-primary ring-offset-2 shadow-md" : ""
      }`}
    >
      <Handle type="target" position={Position.Left} className="!w-2 !h-2 !bg-primary" />
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-1.5">
          <span className={`text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded border ${badgeClass}`}>
            {badge}
          </span>
          <Icon className="w-3.5 h-3.5 opacity-80" />
        </div>
        <div className="font-bold text-xs text-text mt-0.5 leading-tight">{title}</div>
        <div className="text-[10px] text-text-muted leading-tight truncate max-w-[180px]">{subtitle}</div>
      </div>
      <Handle type="source" position={Position.Right} className="!w-2 !h-2 !bg-primary" />
    </div>
  );
}

export function ConfigNode({ data, selected }: NodeProps) {
  const isOverwrite = data?.variant === "overwrite";
  const configKey = typeof data?.configKey === "string" ? data.configKey : "";
  const saveToVariable = typeof data?.saveToVariable === "string" ? data.saveToVariable : "";
  const overrideValue = data?.overrideValue !== undefined ? String(data.overrideValue) : "";

  const badge = isOverwrite ? "CONFIG · GHI ĐÈ" : "CONFIG · ĐỌC";
  const summary = isOverwrite
    ? configKey && overrideValue
      ? `${configKey} → ${overrideValue}`
      : "Chưa cấu hình"
    : configKey && saveToVariable
      ? `${configKey} → ${saveToVariable}`
      : "Chưa cấu hình";

  const subtitle = isOverwrite
    ? "Đọc giá trị gốc trước khi ghi · Tự động khôi phục"
    : "Lưu biến tạm vào ngữ cảnh thi hành";

  const borderClass = isOverwrite
    ? "border-config border-2 bg-config-soft/30"
    : "border-info bg-info-bg";
  const badgeClass = isOverwrite
    ? "bg-config text-white border-config"
    : "bg-info-bg text-info border-info";

  return (
    <div
      className={`min-w-[210px] rounded-2xl border px-3.5 py-2.5 shadow-sm bg-white transition-all cursor-pointer ${borderClass} ${
        selected ? "ring-2 ring-config ring-offset-2 shadow-md" : ""
      }`}
    >
      <Handle type="target" position={Position.Left} className="!w-2 !h-2 !bg-config" />
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-1.5">
          <span className={`text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded border ${badgeClass}`}>
            {badge}
          </span>
          <Database className="w-3.5 h-3.5 text-config" />
        </div>
        <div className="font-bold text-xs text-text mt-0.5 leading-tight">{summary}</div>
        <div className="text-[10px] text-text-muted leading-tight truncate max-w-[200px]">{subtitle}</div>
      </div>
      <Handle type="source" position={Position.Right} className="!w-2 !h-2 !bg-config" />
    </div>
  );
}

export const AUTOMATION_NODE_TYPES = {
  trigger: TriggerNode,
  condition: ConditionNode,
  action: ActionNode,
  config: ConfigNode,
};
