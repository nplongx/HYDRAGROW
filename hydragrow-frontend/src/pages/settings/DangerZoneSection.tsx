import React from 'react';

export interface DangerZoneSectionProps {
  rebootLoading: boolean;
  onReboot: () => void;
  factoryResetConfirm: boolean;
  onFactoryResetClick: () => void;
  onConfirmFactoryReset?: () => void;
  onCancelFactoryReset?: () => void;
}

export const DangerZoneSection: React.FC<DangerZoneSectionProps> = ({
  rebootLoading,
  onReboot,
  factoryResetConfirm,
  onFactoryResetClick,
  onConfirmFactoryReset,
  onCancelFactoryReset,
}) => (
  <section className="mt-8 border-t pt-6">
    <h2 className="text-lg font-semibold text-red-600 mb-4">Vùng Nguy Hiểm</h2>
    <div className="flex gap-3">
      <button
        type="button"
        onClick={onReboot}
        disabled={rebootLoading}
        className="px-4 py-2 border border-orange-400 text-orange-600 rounded-lg text-sm hover:bg-orange-50 disabled:opacity-50"
      >
        Reboot Thiết Bị
      </button>
      <button
        type="button"
        onClick={onFactoryResetClick}
        className="px-4 py-2 border border-red-400 text-red-600 rounded-lg text-sm hover:bg-red-50"
      >
        Factory Reset
      </button>
    </div>
    {factoryResetConfirm && (
      <div className="mt-3 p-4 bg-red-50 rounded-lg">
        <p className="text-sm text-red-700 font-medium mb-3">
          ⚠️ Thao tác này xoá TOÀN BỘ cấu hình (WiFi, recipe, safety budget) và reboot.
          Không thể hoàn tác!
        </p>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={onConfirmFactoryReset}
            className="px-3 py-1.5 bg-red-600 text-white rounded text-sm hover:bg-red-700"
          >
            Xác Nhận Factory Reset
          </button>
          <button
            type="button"
            onClick={onCancelFactoryReset}
            className="px-3 py-1.5 border rounded text-sm hover:bg-white"
          >
            Huỷ
          </button>
        </div>
      </div>
    )}
  </section>
);
