export interface NodePaletteProps {
  onAddNode: (type: 'condition' | 'action') => void;
}

export function NodePalette({ onAddNode }: NodePaletteProps) {
  return (
    <div className="flex gap-2 border-b p-2">
      <button className="rounded bg-amber-100 px-2 py-1 text-xs text-amber-800" onClick={() => onAddNode('condition')}>
        + Condition
      </button>
      <button className="rounded bg-red-100 px-2 py-1 text-xs text-red-800" onClick={() => onAddNode('action')}>
        + Action
      </button>
    </div>
  );
}
