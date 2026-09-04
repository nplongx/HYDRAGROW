import { Plus } from "lucide-react";

interface AutomationPageHeaderProps {
  onNewFlow: () => void;
}

export function AutomationPageHeader({ onNewFlow }: AutomationPageHeaderProps) {
  return (
    <div className="page-header flex justify-between items-center mb-6">
      <h1 className="text-2xl font-bold">Tự động hóa</h1>
      <button
        onClick={onNewFlow}
        className="ui-btn-md ui-btn-primary flex items-center gap-2"
      >
        <Plus className="w-4 h-4" /> Flow mới
      </button>
    </div>
  );
}
