import React, { useState, useEffect } from 'react';
import { Activity, Power, Settings, Droplets, AlertTriangle, ShieldAlert } from 'lucide-react';

// Nếu đường dẫn context của bạn khác, hãy điều chỉnh lại
// import { useDeviceContext } from '../context/DeviceContext'; 

const SystemFlow = () => {
  // Giả lập state (Thay bằng fsmState thật từ MQTT)
  const [fsmState, setFsmState] = useState("DosingPumpA");

  // Dành cho demo (Bạn có thể xóa useEffect này đi khi nối với Backend)
  useEffect(() => {
    const states = ["Monitoring", "WaterRefilling", "DosingPumpA", "WaitingBetweenDose", "DosingPumpB", "DosingPH", "ActiveMixing", "Stabilizing"];
    let i = 0;
    const timer = setInterval(() => {
      i = (i + 1) % states.length;
      setFsmState(states[i]);
    }, 3000);
    return () => clearInterval(timer);
  }, []);

  // --- LOGIC GIAO DIỆN ---
  // Xác định xem bơm nào đang chạy để hiện dòng chảy
  const isA = fsmState === "DosingPumpA";
  const isB = fsmState === "DosingPumpB";
  const isPH = fsmState === "DosingPH";
  const isWaterIn = fsmState === "WaterRefilling";
  const isWaterOut = fsmState === "WaterDraining";
  const isMixing = fsmState === "ActiveMixing";
  const isError = fsmState.includes("Fault") || fsmState === "EmergencyStop";

  // Xác định màu bồn chứa dựa vào trạng thái
  let tankColor = "text-emerald-500"; // Mặc định xanh lá
  if (isA) tankColor = "text-fuchsia-500";
  else if (isB) tankColor = "text-purple-500";
  else if (isPH) tankColor = "text-amber-500";
  else if (isWaterIn || isWaterOut) tankColor = "text-cyan-500";
  else if (isMixing) tankColor = "text-indigo-500";
  else if (isError) tankColor = "text-rose-500";

  return (
    <div className="min-h-screen bg-[#0B1120] text-white p-4 flex flex-col items-center relative overflow-hidden">

      {/* CSS Nhúng cho Animation Dòng chảy (Không cần config Tailwind) */}
      <style>{`
        @keyframes flow-right { to { stroke-dashoffset: -20; } }
        @keyframes flow-left { to { stroke-dashoffset: 20; } }
        @keyframes flow-down { to { stroke-dashoffset: -20; } }
        @keyframes spin-slow { 100% { transform: rotate(360deg); } }
        .animate-flow-right { stroke-dasharray: 8 6; animation: flow-right 0.4s linear infinite; }
        .animate-flow-left { stroke-dasharray: 8 6; animation: flow-left 0.4s linear infinite; }
        .animate-flow-down { stroke-dasharray: 8 6; animation: flow-down 0.4s linear infinite; }
        .mixing-propeller { transform-origin: 200px 220px; animation: spin-slow 1s linear infinite; }
      `}</style>

      {/* Hiệu ứng nền */}
      <div className={`absolute top-0 w-full h-64 ${tankColor.replace('text-', 'bg-')}/10 blur-[100px] transition-colors duration-700`}></div>

      {/* Header Trạng Thái */}
      <div className="z-10 text-center mb-8 mt-4">
        <h2 className="text-sm uppercase tracking-[0.3em] text-slate-400 mb-2">Trạng Thái Trực Tiếp</h2>
        <div className={`inline-flex items-center gap-3 px-6 py-2 rounded-full bg-slate-900 border ${tankColor.replace('text-', 'border-')}/50 shadow-[0_0_20px_rgba(0,0,0,0.5)]`}>
          {isError ? <ShieldAlert className="text-rose-500 animate-ping" /> : <Activity className={`${tankColor}`} />}
          <span className={`text-xl font-bold tracking-wide ${tankColor} transition-colors`}>
            {fsmState.replace("Dosing", "Châm ").replace("Pump", "Phân ").replace("Water", "Nước ")}
          </span>
        </div>
      </div>

      {/* KHOẢNG MÁY (SVG DIGITAL TWIN) - Responsive co giãn cực tốt trên Mobile */}
      <div className="relative w-full max-w-md aspect-square z-10 bg-slate-900/40 rounded-3xl border border-slate-800 backdrop-blur-sm p-4">

        <svg viewBox="0 0 400 400" className="w-full h-full drop-shadow-2xl">
          {/* Định nghĩa Filter Glow */}
          <defs>
            <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="8" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
            <linearGradient id="tank-gradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="currentColor" stopOpacity="0.1" />
              <stop offset="100%" stopColor="currentColor" stopOpacity="0.4" />
            </linearGradient>
          </defs>

          {/* CÁC ĐƯỜNG ỐNG (PIPES) */}
          {/* Ống A */}
          <path d="M 80 120 L 150 120" fill="none" stroke="#334155" strokeWidth="12" strokeLinecap="round" />
          <path d="M 80 120 L 150 120" fill="none" className={isA ? "stroke-fuchsia-400 animate-flow-right" : "hidden"} strokeWidth="6" strokeLinecap="round" filter="url(#glow)" />

          {/* Ống B */}
          <path d="M 80 180 L 150 180" fill="none" stroke="#334155" strokeWidth="12" strokeLinecap="round" />
          <path d="M 80 180 L 150 180" fill="none" className={isB ? "stroke-purple-400 animate-flow-right" : "hidden"} strokeWidth="6" strokeLinecap="round" filter="url(#glow)" />

          {/* Ống pH */}
          <path d="M 80 240 L 150 240" fill="none" stroke="#334155" strokeWidth="12" strokeLinecap="round" />
          <path d="M 80 240 L 150 240" fill="none" className={isPH ? "stroke-amber-400 animate-flow-right" : "hidden"} strokeWidth="6" strokeLinecap="round" filter="url(#glow)" />

          {/* Ống Nước Cấp (Phải qua trái) */}
          <path d="M 320 120 L 250 120" fill="none" stroke="#334155" strokeWidth="16" strokeLinecap="round" />
          <path d="M 320 120 L 250 120" fill="none" className={isWaterIn ? "stroke-cyan-400 animate-flow-left" : "hidden"} strokeWidth="8" strokeLinecap="round" filter="url(#glow)" />

          {/* Ống Xả Đáy */}
          <path d="M 200 300 L 200 360" fill="none" stroke="#334155" strokeWidth="16" strokeLinecap="round" />
          <path d="M 200 300 L 200 360" fill="none" className={isWaterOut ? "stroke-cyan-600 animate-flow-down" : "hidden"} strokeWidth="8" strokeLinecap="round" filter="url(#glow)" />

          {/* BỒN CHỨA (TANK) */}
          <rect x="150" y="80" width="100" height="220" rx="20" fill="#0f172a" stroke="#475569" strokeWidth="4" />
          {/* Mực nước trong bồn (Giả lập mức đầy 60%) */}
          <g className={`${tankColor} transition-all duration-1000`}>
            <rect x="154" y="160" width="92" height="136" rx="16" fill="url(#tank-gradient)" />
            {/* Vạch nước */}
            <line x1="154" y1="160" x2="246" y2="160" stroke="currentColor" strokeWidth="4" filter="url(#glow)" />
          </g>

          {/* Cánh quạt sục trộn (Jet Mixing) */}
          <g className={isMixing ? "mixing-propeller" : "opacity-30"}>
            <circle cx="200" cy="220" r="10" fill="#94a3b8" />
            <path d="M 200 190 L 210 220 L 190 220 Z" fill="#cbd5e1" />
            <path d="M 200 250 L 210 220 L 190 220 Z" fill="#cbd5e1" />
            <path d="M 170 220 L 200 210 L 200 230 Z" fill="#cbd5e1" />
            <path d="M 230 220 L 200 210 L 200 230 Z" fill="#cbd5e1" />
          </g>

          {/* CÁC NÚT BƠM (PUMPS ICON) */}
          {/* Bơm A */}
          <circle cx="60" cy="120" r="24" className={isA ? "fill-fuchsia-900 stroke-fuchsia-400" : "fill-slate-800 stroke-slate-600"} strokeWidth="3" filter={isA ? "url(#glow)" : ""} />
          <text x="60" y="126" textAnchor="middle" fill="white" fontSize="16" fontWeight="bold">A</text>

          {/* Bơm B */}
          <circle cx="60" cy="180" r="24" className={isB ? "fill-purple-900 stroke-purple-400" : "fill-slate-800 stroke-slate-600"} strokeWidth="3" filter={isB ? "url(#glow)" : ""} />
          <text x="60" y="186" textAnchor="middle" fill="white" fontSize="16" fontWeight="bold">B</text>

          {/* Bơm pH */}
          <circle cx="60" cy="240" r="24" className={isPH ? "fill-amber-900 stroke-amber-400" : "fill-slate-800 stroke-slate-600"} strokeWidth="3" filter={isPH ? "url(#glow)" : ""} />
          <text x="60" y="246" textAnchor="middle" fill="white" fontSize="14" fontWeight="bold">pH</text>

          {/* Van Cấp Nước */}
          <circle cx="340" cy="120" r="24" className={isWaterIn ? "fill-cyan-900 stroke-cyan-400" : "fill-slate-800 stroke-slate-600"} strokeWidth="3" filter={isWaterIn ? "url(#glow)" : ""} />
          <text x="340" y="126" textAnchor="middle" fill="white" fontSize="14" fontWeight="bold">H₂O</text>

          {/* Cảm biến EC/pH (Chỉ báo nhỏ đính trên bồn) */}
          <rect x="235" y="240" width="30" height="40" rx="5" className="fill-slate-800 stroke-slate-500" strokeWidth="2" />
          <circle cx="250" cy="250" r="4" fill={fsmState === "Stabilizing" ? "#34d399" : "#64748b"} className={fsmState === "Stabilizing" ? "animate-pulse" : ""} />
          <text x="250" y="270" textAnchor="middle" fill="#94a3b8" fontSize="10" fontWeight="bold">SEN</text>
        </svg>

      </div>

      {/* Legend / Info Panels dưới cùng */}
      <div className="w-full max-w-md grid grid-cols-2 gap-4 mt-8 z-10">
        <div className="bg-slate-900/60 p-4 rounded-2xl border border-slate-800">
          <p className="text-xs text-slate-500 mb-1">Chu trình hoạt động</p>
          <p className="text-sm font-semibold text-slate-200">
            {fsmState === "Monitoring" ? "Đang chờ / Giám sát" : "Đang xử lý tự động"}
          </p>
        </div>
        <div className="bg-slate-900/60 p-4 rounded-2xl border border-slate-800">
          <p className="text-xs text-slate-500 mb-1">Cảnh báo hệ thống</p>
          <p className={`text-sm font-semibold ${isError ? 'text-rose-400' : 'text-emerald-400'}`}>
            {isError ? "Có lỗi xảy ra!" : "Hoạt động bình thường"}
          </p>
        </div>
      </div>

    </div>
  );
};

export default SystemFlow;
