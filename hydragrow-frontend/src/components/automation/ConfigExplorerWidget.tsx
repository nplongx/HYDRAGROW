import { ArrowRight } from "lucide-react";
import type { ConfigOverrideActiveItem } from "../../types/automation";

interface Props {
  items?: ConfigOverrideActiveItem[];
  onOpenFullView: () => void;
}

export function ConfigExplorerWidget({ items = [], onOpenFullView }: Props) {
  return (
    <div className="bg-white rounded-2xl border border-line p-5 shadow-low flex flex-col justify-between h-full">
      <div>
        <div className="flex items-center justify-between mb-1">
          <h3 className="text-xs font-bold text-config tracking-wider uppercase flex items-center gap-1.5">
            CONFIG EXPLORER
            <span className="bg-config text-white text-[10px] font-semibold px-1.5 py-0.2 rounded">
              MỚI
            </span>
          </h3>
        </div>
        <p className="text-xs text-text-secondary mb-4 leading-relaxed">
          Xem trực tiếp giá trị config đang chạy trên thiết bị và Flow nào đang ghi đè nó.
        </p>

        {items.length === 0 && (
          <div className="py-6 text-center text-xs text-text-muted bg-config-soft/30 rounded-xl border border-dashed border-line p-4 mb-2">
            <span className="block font-medium text-config mb-1">Giá trị gốc ổn định</span>
            Tất cả thông số đang hoạt động theo công thức mặc định. Chưa có Flow nào ghi đè cấu hình.
          </div>
        )}

        <div className="divide-y divide-line">
          {items.map((item) => {
            const isOverridden = item.status === "active";
            return (
              <div key={`${item.configKey}-${item.deviceId}`} className="py-3 flex items-center justify-between">
                <div>
                  <span className="font-mono text-xs font-semibold text-text-primary block">
                    {item.configKey}
                  </span>
                  <span className="text-[11px] text-text-muted block mt-0.5">
                    {isOverridden ? `Ghi đè bởi: ${item.flowName}` : item.flowName}
                  </span>
                </div>

                <div className="text-right">
                  <span
                    className={`font-semibold text-sm ${
                      isOverridden ? "text-config font-bold" : "text-text-secondary"
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
        className="mt-4 w-full ui-btn-primary text-xs cursor-pointer"
      >
        <span>Xem toàn bộ nhật ký Config</span>
        <ArrowRight className="w-3.5 h-3.5" />
      </button>
    </div>
  );
}
