import React from 'react';
import { LockKeyhole, Power } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Switch } from '../../components/ui/Switch';

interface GeneralSectionProps {
  userEmail: string | null | undefined;
  onLogout: () => void;
  onGoToPairing?: () => void;
  isAdvancedMode: boolean;
  onToggleAdvancedMode: (value: boolean) => void;
  controlMode?: 'auto' | 'manual';
  onControlModeChange?: (value: 'auto' | 'manual') => void;
}

export const GeneralSection: React.FC<GeneralSectionProps> = ({
  userEmail,
  onLogout,
  isAdvancedMode,
  onToggleAdvancedMode,
  controlMode = 'auto',
  onControlModeChange,
}) => {
  const navigate = useNavigate();

  return (
    <div className="space-y-4">
      <div className="ui-card space-y-3">
        <h3 className="farm-section-title">Tài khoản đăng nhập</h3>
        <p className="text-sm text-text-muted">
          Đang đăng nhập: <strong>{userEmail ?? 'Không xác định'}</strong>
        </p>
        <button
          type="button"
          onClick={onLogout}
          className="ui-btn-md border border-line text-primary-deep bg-white hover:bg-soft"
        >
          Đăng xuất
        </button>
      </div>

      <div className="ui-card space-y-3">
        <div className="flex items-center gap-2">
          <Power size={17} className="text-primary" />
          <h3 className="farm-section-title">Chế độ hoạt động</h3>
        </div>
        <p className="text-xs text-text-muted">
          Chọn cách hệ thống điều khiển thiết bị. Tự động dùng các ngưỡng đã cấu hình; Thủ công cho phép điều khiển trực tiếp.
        </p>
        <div className="grid grid-cols-2 gap-2" role="group" aria-label="Chế độ hoạt động">
          {(['auto', 'manual'] as const).map((mode) => {
            const selected = controlMode === mode;
            return (
              <button
                key={mode}
                type="button"
                aria-pressed={selected}
                onClick={() => onControlModeChange?.(mode)}
                className={`rounded-xl border px-4 py-3 text-sm font-semibold transition-colors ${
                  selected
                    ? 'border-primary bg-primary text-white'
                    : 'border-line bg-white text-primary-deep hover:bg-soft'
                }`}
              >
                {mode === 'auto' ? 'Tự động' : 'Thủ công'}
              </button>
            );
          })}
        </div>
      </div>

      <div className="ui-card space-y-3">
        <h3 className="farm-section-title">Ghép nối thiết bị</h3>
        <button
          type="button"
          onClick={() => navigate('/pairing')}
          className="ui-btn-primary w-full flex items-center justify-center gap-2"
        >
          Ghép thiết bị mới
        </button>
      </div>

      <div
        className={`ui-card flex items-center justify-between gap-4 ${
          isAdvancedMode ? 'bg-amber-50 border-amber-200' : ''
        }`}
      >
        <div className="flex items-center gap-3">
          <div
            className={`p-2 rounded-lg ${
              isAdvancedMode ? 'bg-amber-100 text-amber-800' : 'bg-pill text-status'
            }`}
          >
            <LockKeyhole size={16} />
          </div>
          <div>
            <p className="text-sm font-semibold text-primary-deep">Chế độ kỹ thuật</p>
            <p className="text-[11px] text-text-muted">Mở rộng thông số an toàn & hiệu chuẩn</p>
          </div>
        </div>
        <Switch isOn={isAdvancedMode} onClick={onToggleAdvancedMode} colorClass="bg-amber-600" />
      </div>
    </div>
  );
};