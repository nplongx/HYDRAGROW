import React from 'react';

export const FsmStatusBadge: React.FC<{ state?: string }> = ({ state }) => {
  const rawState = state || "Monitoring";

  // Hàm render giao diện cho thẻ Tag bằng Tailwind CSS
  const renderBadge = (bgClass: string, textClass: string, content: string) => (
    <span className={`px-3 py-1 rounded-full text-xs font-bold shadow-sm ${bgClass} ${textClass}`}>
      {content}
    </span>
  );

  // Xử lý báo lỗi hệ thống có kèm lý do
  if (rawState.startsWith("SystemFault:")) {
    const reason = rawState.replace("SystemFault:", "");
    return renderBadge("bg-red-500/20 border border-red-500/50", "text-red-400", `🚨 Lỗi: ${reason}`);
  }

  // 🟢 MỚI: Xử lý Dừng Khẩn Cấp có kèm lý do (Khớp với code ESP32 mới)
  if (rawState.startsWith("EmergencyStop:")) {
    const reason = rawState.replace("EmergencyStop:", "");
    return renderBadge("bg-red-600 border border-red-500 animate-pulse", "text-white", `🛑 DỪNG KHẨN CẤP: ${reason}`);
  }

  switch (rawState) {
    case "Monitoring": return renderBadge("bg-emerald-500/20 border border-emerald-500/50", "text-emerald-400", "🟢 Đang Giám Sát");
    case "EmergencyStop": return renderBadge("bg-red-600 border border-red-500", "text-white", "🛑 DỪNG KHẨN CẤP"); // Fallback cho code cũ
    case "WaterRefilling": return renderBadge("bg-blue-500/20 border border-blue-500/50", "text-blue-400", "💧 Đang Cấp Nước");
    case "WaterDraining": return renderBadge("bg-cyan-500/20 border border-cyan-500/50", "text-cyan-400", "🌊 Đang Xả Nước");
    case "DosingPumpA": return renderBadge("bg-orange-500/20 border border-orange-500/50", "text-orange-400", "🧪 Đang Châm Phân A");
    case "WaitingBetweenDose": return renderBadge("bg-slate-500/20 border border-slate-500/50", "text-slate-300", "⏳ Chờ Hòa Tan");
    case "DosingPumpB": return renderBadge("bg-orange-500/20 border border-orange-500/50", "text-orange-400", "🧪 Đang Châm Phân B");
    case "DosingPH": return renderBadge("bg-purple-500/20 border border-purple-500/50", "text-purple-400", "⚖️ Đang Chỉnh pH");
    case "StartingOsakaPump": return renderBadge("bg-indigo-500/20 border border-indigo-500/50", "text-indigo-400", "⚙️ Khởi Động Bơm");
    case "ActiveMixing": return renderBadge("bg-sky-500/20 border border-sky-500/50", "text-sky-400", "🌪️ Đang Sục Trộn");
    case "Stabilizing": return renderBadge("bg-yellow-500/20 border border-yellow-500/50", "text-yellow-400", "⚖️ Chờ Ổn Định");
    case "Disconnected": return renderBadge("bg-red-500/20 border border-red-500/50", "text-red-500", "🔴 Mất Kết Nối");
    case "Offline": return renderBadge("bg-slate-500/20 border border-slate-500/50", "text-slate-400", "🔌 Mất Kết Nối");

    // Trường hợp không có trong từ điển (Fallback)
    default: return renderBadge("bg-slate-700 border border-slate-600", "text-slate-300", rawState);
  }
};
