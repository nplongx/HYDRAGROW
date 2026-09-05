import { ArrowRight } from "lucide-react";
import type { ConfigOverrideActiveItem } from "../../types/automation";

interface Props {
  items?: ConfigOverrideActiveItem[];
  onOpenFullView: () => void;
}

const DEFAULT_CONFIGS: ConfigOverrideActiveItem[] = [
  {
    configKey: "ec_target",
    deviceId: "dev-01",
    originalValue: "2.4",
    currentValue: "1.8",
    unit: "mS/cm",
    flowName: "Hạ ngưỡng EC ban đêm",
    status: "active",
  },
  {
    configKey: "ph_target",
    deviceId: "dev-01",
    originalValue: "6.2",
    currentValue: "6.2",
    unit: "",
    flowName: "Giá trị gốc từ Recipe",
    status: "restored",
  },
  {
    configKey: "dose_max_ml",
    deviceId: "dev-01",
    originalValue: "15",
    currentValue: "15",
    unit: "ml",
    flowName: "Giá trị gốc từ Recipe",
    status: "restored",
  },
  {
    configKey: "water_cycle_sec",
    deviceId: "dev-01",
    originalValue: "1800",
    currentValue: "1800",
    unit: "s",
    flowName: "Giới hạn dosing (đã tắt)",
    status: "restored",
  },
];

export function ConfigExplorerWidget({ items = DEFAULT_CONFIGS, onOpenFullView }: Props) {
  return (
    <div className="bg-white rounded-2xl border border-indigo-100 p-5 shadow-sm flex flex-col justify-between h-full">
      <div>
        <div className="flex items-center justify-between mb-1">
          <h3 className="text-xs font-bold text-indigo-900 tracking-wider uppercase flex items-center gap-1.5">
            CONFIG EXPLORER
            <span className="bg-indigo-600 text-white text-[10px] font-semibold px-1.5 py-0.2 rounded">
              MỚI
            </span>
          </h3>
        </div>
        <p className="text-xs text-indigo-950/70 mb-4 leading-relaxed">
          Xem trực tiếp giá trị config đang chạy trên thiết bị và Flow nào đang ghi đè nó.
        </p>

        <div className="divide-y divide-indigo-50/80">
          {items.map((item) => {
            const isOverridden = item.status === "active";
            return (
              <div key={item.configKey} className="py-3 flex items-center justify-between">
                <div>
                  <span className="font-mono text-xs font-semibold text-indigo-950 block">
                    {item.configKey}
                  </span>
                  <span className="text-[11px] text-indigo-800/60 block mt-0.5">
                    {isOverridden ? `Ghi đè bởi: ${item.flowName}` : item.flowName}
                  </span>
                </div>

                <div className="text-right">
                  <span
                    className={`font-semibold text-sm ${
                      isOverridden ? "text-indigo-700 font-bold" : "text-slate-700"
                    }`}
                  >
                    {item.currentValue} {item.unit}
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <button
        type="button"
        onClick={onOpenFullView}
        className="mt-4 w-full inline-flex items-center justify-center gap-2 rounded-xl bg-indigo-600 px-4 py-2.5 text-xs font-semibold text-white shadow-sm hover:bg-indigo-700 transition-colors cursor-pointer"
      >
        <span>Xem toàn bộ nhật ký Config</span>
        <ArrowRight className="w-3.5 h-3.5" />
      </button>
    </div>
  );
}
