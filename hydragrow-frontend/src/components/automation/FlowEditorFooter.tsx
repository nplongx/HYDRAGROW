import { Play } from "lucide-react";

interface FlowEditorFooterProps {
  isNew: boolean;
  pending: boolean;
  onDelete: () => void;
  onTest: () => void;
  onSave: () => void;
}

export function FlowEditorFooter({
  isNew,
  pending,
  onDelete,
  onTest,
  onSave,
}: FlowEditorFooterProps) {
  return (
    <div className="flex justify-between mt-4">
      {!isNew ? (
        <button
          onClick={onDelete}
          disabled={pending}
          className="rounded-lg bg-red-50 border border-red-200 px-3 py-1.5 text-sm font-semibold text-red-700"
        >
          Xóa Flow
        </button>
      ) : (
        <span />
      )}
      <div className="flex gap-2">
        <button
          onClick={onTest}
          disabled={pending}
          className="ui-btn-md flex items-center gap-2 bg-white border text-slate-700"
        >
          <Play className="h-4 w-4" /> Chạy thử
        </button>
        <button onClick={onSave} disabled={pending} className="ui-btn-primary">
          Lưu Flow
        </button>
      </div>
    </div>
  );
}
