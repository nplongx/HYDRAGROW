import React from 'react';

interface InputGroupProps {
  label: string;
  type?: string;
  value: string | number;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  step?: string;
  desc?: string;
}

export const InputGroup: React.FC<InputGroupProps> = ({ label, type = "number", value, onChange, step, desc }) => (
  <div className="flex flex-col space-y-1.5 group">
    <label className="text-[10px] font-bold text-slate-500 group-focus-within:text-emerald-400 uppercase tracking-wider pl-1 transition-colors">
      {label}
    </label>
    <input
      type={type} step={step} value={value} onChange={onChange}
      className="bg-slate-950/50 border border-slate-800 text-white text-sm rounded-xl px-4 py-2.5 focus:outline-none focus:ring-2 focus:ring-emerald-500/30 focus:border-emerald-500/50 transition-all hover:border-slate-700"
    />
    {desc && <span className="text-[10px] text-slate-500 pl-1 leading-tight">{desc}</span>}
  </div>
);
