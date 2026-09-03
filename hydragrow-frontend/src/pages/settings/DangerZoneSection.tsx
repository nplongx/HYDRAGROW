import React from 'react';
import { AlertTriangle, RotateCcw, Trash2 } from 'lucide-react';

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
  <section className="rounded-2xl border border-red-200 bg-red-50/60 p-4 md:p-5 space-y-3">
    <h3 className="farm-section-title text-red-700">
      <AlertTriangle size={14} />
      <span>Vùng nguy hiểm</span>
    </h3>

    <div className="flex flex-col sm:flex-row gap-3">
      <button
        type="button"
        onClick={onReboot}
        disabled={rebootLoading}
        className="ui-btn-md flex items-center justify-center gap-2 border border-amber-300 bg-white text-amber-700 hover:bg-amber-50"
      >
        <RotateCcw size={15} className={rebootLoading ? 'animate-spin' : ''} />
        {rebootLoading ? 'Đang gửi lệnh...' : 'Reboot thiết bị'}
      </button>
      <button
        type="button"
        onClick={onFactoryResetClick}
        className="ui-btn-md flex items-center justify-center gap-2 border border-red-300 bg-white text-red-700 hover:bg-red-50"
      >
        <Trash2 size={15} />
        Factory Reset
      </button>
    </div>
    <p className="text-[11px] text-red-700/80 leading-relaxed">
      Factory Reset sẽ xoá toàn bộ cấu hình (WiFi, recipe, safety budget) và không thể hoàn tác.
    </p>

    {factoryResetConfirm && (
      <div className="rounded-xl border border-red-300 bg-white p-4 space-y-3">
        <p className="text-sm font-semibold text-red-700 leading-relaxed">
          Xác nhận: thao tác này xoá toàn bộ cấu hình (WiFi, recipe, safety budget) và khởi động lại thiết bị. Không thể hoàn tác.
        </p>
        <div className="flex flex-col sm:flex-row gap-2">
          <button
            type="button"
            onClick={onConfirmFactoryReset}
            className="ui-btn-md bg-red-600 text-white hover:bg-red-700"
          >
            Xác Nhận Factory Reset
          </button>
          <button
            type="button"
            onClick={onCancelFactoryReset}
            className="ui-btn-md border border-emerald-200 bg-white text-emerald-800 hover:bg-emerald-50"
          >
            Huỷ
          </button>
        </div>
      </div>
    )}
  </section>
);
