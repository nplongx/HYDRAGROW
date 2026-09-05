import { useState, useMemo } from "react";
import { Search, ArrowLeft, ShieldCheck, AlertTriangle } from "lucide-react";
import type { ConfigOverrideActiveItem, ConfigAuditLogEntry } from "../../types/automation";

interface Props {
  onBack: () => void;
}

const SAMPLE_OVERRIDES: ConfigOverrideActiveItem[] = [
  {
    configKey: "ec_target",
    deviceId: "dev-a1",
    deviceName: "Nhà kính A · Kệ 1",
    originalValue: "2.4",
    currentValue: "1.8",
    unit: "mS/cm",
    flowName: "Hạ ngưỡng EC ban đêm",
    status: "active",
  },
  {
    configKey: "ec_target",
    deviceId: "dev-a2",
    deviceName: "Nhà kính A · Kệ 2",
    originalValue: "2.4",
    currentValue: "1.8",
    unit: "mS/cm",
    flowName: "Hạ ngưỡng EC ban đêm",
    status: "active",
  },
  {
    configKey: "ph_target",
    deviceId: "dev-b1",
    deviceName: "Nhà kính B · Kệ 1",
    originalValue: "6.2",
    currentValue: "5.8",
    unit: "",
    flowName: "Bù pH giai đoạn ra hoa",
    status: "active",
  },
  {
    configKey: "dose_max_ml",
    deviceId: "dev-b2",
    deviceName: "Nhà kính B · Kệ 2",
    originalValue: "15",
    currentValue: "15",
    unit: "ml",
    flowName: "Giới hạn dosing khi cảm biến lỗi",
    status: "restored",
  },
  {
    configKey: "water_cycle_sec",
    deviceId: "dev-c1",
    deviceName: "Nhà kính C · Kệ 1",
    originalValue: "1800",
    currentValue: "2400",
    unit: "s",
    flowName: "Tưới thừa mùa mưa",
    status: "active",
  },
  {
    configKey: "ph_target",
    deviceId: "dev-c2",
    deviceName: "Nhà kính C · Kệ 2",
    originalValue: "6.2",
    currentValue: "6.2",
    unit: "",
    flowName: "Bù pH giai đoạn ra hoa",
    status: "restored",
  },
];

const SAMPLE_AUDIT_LOGS: ConfigAuditLogEntry[] = [
  {
    id: "log-1",
    timestamp: "05/09 22:00",
    deviceId: "dev-a1",
    deviceName: "Nhà kính A · Kệ 1",
    configKey: "ec_target",
    originalValue: "2.4",
    overrideValue: "1.8",
    unit: "mS/cm",
    reason: 'Điều kiện "Khung giờ ban đêm" chuyển sang đúng',
    status: "applied",
  },
  {
    id: "log-2",
    timestamp: "05/09 05:00",
    deviceId: "dev-a1",
    deviceName: "Nhà kính A · Kệ 1",
    configKey: "ec_target",
    originalValue: "1.8",
    overrideValue: "2.4",
    unit: "mS/cm",
    reason: "Điều kiện sai -> tự khôi phục giá trị gốc",
    status: "restored",
  },
  {
    id: "log-3",
    timestamp: "05/09 21:58",
    deviceId: "dev-b1",
    deviceName: "Nhà kính B · Kệ 1",
    configKey: "ph_target",
    originalValue: "6.2",
    overrideValue: "5.8",
    unit: "",
    reason: "Webhook ngoài: cảm biến hoa báo cần hạ pH",
    status: "applied",
  },
  {
    id: "log-4",
    timestamp: "04/09 22:00",
    deviceId: "dev-c1",
    deviceName: "Nhà kính C · Kệ 1",
    configKey: "water_cycle_sec",
    originalValue: "1800",
    overrideValue: "2400",
    unit: "s",
    reason: "Đề xuất vượt Max cho phép -> đã kẹp về 2400s",
    status: "clamped_warning",
  },
];

export function ConfigExplorerView({ onBack }: Props) {
  const [searchTerm, setSearchTerm] = useState("");
  const [selectedKey, setSelectedKey] = useState<string>("all");

  const filteredOverrides = useMemo(() => {
    return SAMPLE_OVERRIDES.filter((item) => {
      const matchKey = selectedKey === "all" || item.configKey === selectedKey;
      const matchSearch =
        searchTerm === "" ||
        item.configKey.toLowerCase().includes(searchTerm.toLowerCase()) ||
        item.deviceName?.toLowerCase().includes(searchTerm.toLowerCase()) ||
        item.flowName.toLowerCase().includes(searchTerm.toLowerCase());
      return matchKey && matchSearch;
    });
  }, [selectedKey, searchTerm]);

  return (
    <div className="space-y-6">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 mb-1">
            <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded-full bg-indigo-100 text-indigo-800 border border-indigo-200">
              TÍNH NĂNG MỚI
            </span>
          </div>
          <h2 className="text-2xl font-bold text-emerald-950">
            Config Explorer & Nhật ký ghi đè
          </h2>
          <p className="text-sm text-emerald-800/70 mt-1 max-w-3xl">
            Tra cứu toàn bộ giá trị config đang bị Flow ghi đè trên mọi thiết bị, lọc theo Flow/thiết bị/khóa config, và xem nhật ký thay đổi xuyên suốt hệ thống — không cần mở từng node.
          </p>
        </div>

        <button
          type="button"
          onClick={onBack}
          className="inline-flex items-center gap-2 rounded-xl border border-emerald-200 bg-white px-4 py-2 text-sm font-semibold text-emerald-900 shadow-sm hover:bg-emerald-50 transition-colors self-start sm:self-auto cursor-pointer"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>Quay lại Flow</span>
        </button>
      </div>

      {/* KPI Stats */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-white rounded-2xl border border-indigo-100 p-4 shadow-sm">
          <div className="text-3xl font-bold text-indigo-700">9</div>
          <div className="text-xs text-indigo-900/70 font-medium mt-1">Config key đang bị ghi đè</div>
        </div>
        <div className="bg-white rounded-2xl border border-emerald-100 p-4 shadow-sm">
          <div className="text-3xl font-bold text-emerald-800">6</div>
          <div className="text-xs text-emerald-900/70 font-medium mt-1">Thiết bị có override cục bộ</div>
        </div>
        <div className="bg-white rounded-2xl border border-amber-100 p-4 shadow-sm">
          <div className="text-3xl font-bold text-amber-700">14</div>
          <div className="text-xs text-amber-900/70 font-medium mt-1">Lượt ghi đè trong 24h</div>
        </div>
        <div className="bg-white rounded-2xl border border-teal-100 p-4 shadow-sm">
          <div className="text-3xl font-bold text-teal-700">96%</div>
          <div className="text-xs text-teal-900/70 font-medium mt-1">Tự khôi phục đúng điều kiện</div>
        </div>
      </div>

      {/* Search & Filter Bar */}
      <div className="bg-white p-3 rounded-2xl border border-emerald-100 flex flex-col md:flex-row items-center gap-3 shadow-sm">
        <div className="relative flex-1 w-full">
          <Search className="w-4 h-4 text-emerald-700/50 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            placeholder="Tìm theo config key, thiết bị hoặc tên Flow..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="ui-input pl-9 w-full text-sm"
          />
        </div>

        <div className="flex flex-wrap items-center gap-1.5 w-full md:w-auto">
          {[
            { id: "all", label: "Tất cả 9" },
            { id: "ec_target", label: "ec_target 3" },
            { id: "ph_target", label: "ph_target 2" },
            { id: "dose_max_ml", label: "dose_max_ml 2" },
            { id: "water_cycle_sec", label: "water_cycle_sec 2" },
          ].map((pill) => (
            <button
              key={pill.id}
              type="button"
              onClick={() => setSelectedKey(pill.id)}
              className={`px-3 py-1 rounded-lg text-xs font-semibold transition-colors cursor-pointer ${
                selectedKey === pill.id
                  ? "bg-emerald-700 text-white shadow-sm"
                  : "bg-emerald-50 text-emerald-800 hover:bg-emerald-100"
              }`}
            >
              {pill.label}
            </button>
          ))}
        </div>
      </div>

      {/* Active Overrides Table */}
      <div className="bg-white rounded-2xl border border-emerald-100 overflow-hidden shadow-sm">
        <div className="px-5 py-3.5 border-b border-emerald-50 bg-emerald-50/40">
          <h3 className="font-semibold text-emerald-950 text-sm">
            Danh sách Config đang hoạt động & Flow kiểm soát
          </h3>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse text-xs">
            <thead>
              <tr className="border-b border-emerald-100 bg-slate-50/60 text-slate-500 font-semibold uppercase tracking-wider">
                <th className="py-3 px-4">CONFIG KEY</th>
                <th className="py-3 px-4">THIẾT BỊ</th>
                <th className="py-3 px-4">GIÁ TRỊ GỐC</th>
                <th className="py-3 px-4">HIỆN TẠI</th>
                <th className="py-3 px-4">FLOW ĐANG GIỮ</th>
                <th className="py-3 px-4">TRẠNG THÁI</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-emerald-50">
              {filteredOverrides.map((row, idx) => (
                <tr key={`${row.deviceId}-${row.configKey}-${idx}`} className="hover:bg-emerald-50/30">
                  <td className="py-3 px-4 font-mono font-bold text-emerald-950">{row.configKey}</td>
                  <td className="py-3 px-4 text-slate-700">{row.deviceName}</td>
                  <td className="py-3 px-4 text-slate-500">{row.originalValue} {row.unit}</td>
                  <td className="py-3 px-4 font-bold text-indigo-700">
                    {row.currentValue} {row.unit}
                  </td>
                  <td className="py-3 px-4">
                    <span className="inline-flex items-center gap-1 font-medium text-emerald-900">
                      <span className="text-[10px] font-bold px-1 rounded bg-indigo-100 text-indigo-800">CONFIG</span>
                      {row.flowName}
                    </span>
                  </td>
                  <td className="py-3 px-4">
                    {row.status === "active" ? (
                      <span className="px-2.5 py-0.5 rounded-full text-[11px] font-semibold bg-amber-100 text-amber-800 border border-amber-200">
                        Đang override
                      </span>
                    ) : (
                      <span className="px-2.5 py-0.5 rounded-full text-[11px] font-semibold bg-emerald-100 text-emerald-800 border border-emerald-200">
                        Đã khôi phục
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Audit Log Table */}
      <div className="bg-white rounded-2xl border border-emerald-100 overflow-hidden shadow-sm">
        <div className="px-5 py-3.5 border-b border-emerald-50 bg-emerald-50/40 flex items-center justify-between">
          <h3 className="font-semibold text-emerald-950 text-sm flex items-center gap-2">
            Nhật ký ghi đè toàn hệ thống (audit log) — xuyên suốt mọi thiết bị
            <span className="bg-emerald-600 text-white text-[10px] font-semibold px-1.5 py-0.2 rounded">
              MỚI
            </span>
          </h3>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse text-xs">
            <thead>
              <tr className="border-b border-emerald-100 bg-slate-50/60 text-slate-500 font-semibold uppercase tracking-wider">
                <th className="py-3 px-4">THỜI GIAN</th>
                <th className="py-3 px-4">THIẾT BỊ</th>
                <th className="py-3 px-4">CONFIG KEY</th>
                <th className="py-3 px-4">GỐC &rarr; GHI ĐÈ</th>
                <th className="py-3 px-4">LÝ DO KÍCH HOẠT</th>
                <th className="py-3 px-4">TRẠNG THÁI</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-emerald-50">
              {SAMPLE_AUDIT_LOGS.map((log) => (
                <tr key={log.id} className="hover:bg-emerald-50/30">
                  <td className="py-3 px-4 font-mono text-slate-500">{log.timestamp}</td>
                  <td className="py-3 px-4 font-medium text-slate-800">{log.deviceName}</td>
                  <td className="py-3 px-4 font-mono font-bold text-indigo-900">{log.configKey}</td>
                  <td className="py-3 px-4 font-medium text-slate-700">
                    {log.originalValue} &rarr; <span className="font-bold text-indigo-700">{log.overrideValue}</span> {log.unit}
                  </td>
                  <td className="py-3 px-4 text-slate-600">{log.reason}</td>
                  <td className="py-3 px-4">
                    {log.status === "applied" && (
                      <span className="inline-flex items-center gap-1 font-semibold text-emerald-700">
                        <ShieldCheck className="w-3.5 h-3.5" />
                        Đã áp dụng
                      </span>
                    )}
                    {log.status === "restored" && (
                      <span className="inline-flex items-center gap-1 font-semibold text-sky-700">
                        Đã khôi phục
                      </span>
                    )}
                    {log.status === "clamped_warning" && (
                      <span className="inline-flex items-center gap-1 font-semibold text-amber-700">
                        <AlertTriangle className="w-3.5 h-3.5" />
                        Cảnh báo - Đã kẹp
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
