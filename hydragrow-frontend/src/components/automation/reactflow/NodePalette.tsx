export interface NodePaletteProps {
  onAddNode: (type: 'condition' | 'condition_group' | 'action') => void;
}

export function NodePalette({ onAddNode }: NodePaletteProps) {
  return (
    <div className="flex gap-2 border-b border-emerald-100 p-2">
      <button className="rounded-lg bg-amber-50 border border-amber-200 px-2 py-1 text-xs font-semibold text-amber-800" onClick={() => onAddNode('condition')}>
        + Condition
      </button>
      <button className="rounded-lg bg-amber-50 border border-amber-200 px-2 py-1 text-xs font-semibold text-amber-800" onClick={() => onAddNode('condition_group')}>
        + Condition Group
      </button>
      <button className="rounded-lg bg-emerald-50 border border-emerald-200 px-2 py-1 text-xs font-semibold text-emerald-800" onClick={() => onAddNode('action')}>
        + Action
      </button>
    </div>
  );
}
