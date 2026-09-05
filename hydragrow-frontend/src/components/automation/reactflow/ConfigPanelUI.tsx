import type { ReactNode } from "react";

export type Tone = "sky" | "amber" | "indigo" | "emerald";

const BADGE_TONE: Record<Tone, string> = {
  sky: "bg-sky-100 text-sky-700",
  amber: "bg-amber-50 text-amber-800",
  indigo: "bg-indigo-50 text-indigo-700",
  emerald: "bg-emerald-50 text-emerald-700",
};

const CARD_BORDER_TONE: Record<Tone, string> = {
  sky: "border-emerald-100",
  amber: "border-emerald-100",
  indigo: "border-indigo-200",
  emerald: "border-emerald-100",
};

const CARD_EMPHASIZED_BORDER_TONE: Record<Tone, string> = {
  sky: "border-sky-600",
  amber: "border-amber-600",
  indigo: "border-indigo-600",
  emerald: "border-emerald-600",
};

export function Badge({ tone, children }: { tone: Tone; children: ReactNode }) {
  return (
    <span
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-[9.5px] font-bold ${BADGE_TONE[tone]}`}
    >
      {children}
    </span>
  );
}

export function ConfigCard({
  tone,
  emphasized,
  children,
}: {
  tone: Tone;
  emphasized?: boolean;
  children: ReactNode;
}) {
  const border = emphasized
    ? `border-[1.5px] ${CARD_EMPHASIZED_BORDER_TONE[tone]}`
    : `border ${CARD_BORDER_TONE[tone]}`;
  return (
    <div className={`flex flex-col gap-3 rounded-2xl bg-white p-4 ${border}`}>
      {children}
    </div>
  );
}

export function FieldGroup({
  label,
  children,
  htmlFor,
}: {
  label: string;
  children: ReactNode;
  htmlFor?: string;
}) {
  return (
    <label htmlFor={htmlFor} className="flex flex-col gap-1.5">
      <span className="text-[11px] text-emerald-800/70">{label}</span>
      {children}
    </label>
  );
}

const CHIP_TONE: Record<Tone, string> = {
  sky: "bg-sky-100 text-sky-700",
  amber: "bg-amber-100 text-amber-800",
  indigo: "bg-indigo-100 text-indigo-700",
  emerald: "bg-emerald-50 text-emerald-700",
};

export function Chip({
  tone = "emerald",
  children,
  onRemove,
}: {
  tone?: Tone;
  children: ReactNode;
  onRemove?: () => void;
}) {
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[10.5px] font-semibold ${CHIP_TONE[tone]}`}
    >
      {children}
      {onRemove && (
        <button type="button" aria-label="Xóa" className="font-bold" onClick={onRemove}>
          ×
        </button>
      )}
    </span>
  );
}

export function ChipsRow({ children }: { children: ReactNode }) {
  return <div className="flex flex-wrap items-center gap-1.5">{children}</div>;
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
}: {
  options: { value: T; label: string }[];
  value: T;
  onChange: (v: T) => void;
}) {
  return (
    <div className="flex gap-0.5 rounded-[10px] border border-emerald-100 p-0.5">
      {options.map((opt) => (
        <button
          key={opt.value}
          type="button"
          aria-pressed={value === opt.value}
          className={`flex-1 rounded-lg px-3.5 py-1.5 text-[11.5px] font-semibold transition-colors ${
            value === opt.value
              ? "bg-emerald-700 text-white"
              : "text-emerald-800/70 hover:bg-emerald-50"
          }`}
          onClick={() => onChange(opt.value)}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}

export function ToggleRow({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex w-full items-center justify-between gap-3 cursor-pointer">
      <span className="text-[11.5px] text-emerald-950">{label}</span>
      <input
        type="checkbox"
        role="switch"
        aria-label={label}
        aria-checked={checked}
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="toggle-checkbox shrink-0 scale-90"
      />
    </label>
  );
}

export function SafeNote({ children }: { children: ReactNode }) {
  return (
    <div className="flex items-center gap-1.5 rounded-lg bg-emerald-50 px-2.5 py-1.5 text-[11px] font-medium text-emerald-700">
      <span className="font-bold">✓</span>
      <span>{children}</span>
    </div>
  );
}
