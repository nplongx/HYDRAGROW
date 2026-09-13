interface SparklineProps {
  values: number[];
  label: string;
  strokeClassName?: string;
}

/** Sparkline inline tối giản — data-visualization skill: "Trend Over Time -> sparklines (inline)". */
export const Sparkline = ({ values, label, strokeClassName = 'stroke-primary' }: SparklineProps) => {
  if (values.length < 2) return null;

  const width = 100;
  const height = 24;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min || 1;

  const points = values
    .map((v, i) => {
      const x = (i / (values.length - 1)) * width;
      const y = height - ((v - min) / range) * height;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(' ');

  return (
    <svg
      role="img"
      aria-label={`Xu hướng ${label} trong phiên hiện tại, ${values.length} mẫu gần nhất`}
      viewBox={`0 0 ${width} ${height}`}
      className="w-full h-6"
      preserveAspectRatio="none"
    >
      <polyline
        points={points}
        fill="none"
        className={strokeClassName}
        strokeWidth={1.5}
        strokeLinejoin="round"
        strokeLinecap="round"
      />
    </svg>
  );
};
