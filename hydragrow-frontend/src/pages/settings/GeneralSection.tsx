import React from 'react';
import { LockKeyhole } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Switch } from '../../components/ui/Switch';

interface GeneralSectionProps {
  userEmail: string | null | undefined;
  onLogout: () => void;
  onGoToPairing: () => void;
  isAdvancedMode: boolean;
  onToggleAdvancedMode: (value: boolean) => void;
}

export const GeneralSection: React.FC<GeneralSectionProps> = ({
  userEmail,
  onLogout,
  onGoToPairing,
  isAdvancedMode,
  onToggleAdvancedMode,
}) => {
  const navigate = useNavigate();

  const goToPairing = () => {
    // Use React Router navigation. Hard reload via window.location.href breaks
    // BrowserRouter/Tauri deep links on /pairing in deployed builds.
    onGoToPairing();
    navigate('/pairing');
  };

  return (
    <div className="space-y-4">
      <div className="ui-card space-y-3">
        <h3 className="farm-section-title">Tài khoản đăng nhập</h3>
        <p className="text-sm text-emerald-800/80">
          Đang đăng nhập: <strong>{userEmail ?? 'Không xác định'}</strong>
        </p>
        <button
          type="button"
          onClick={onLogout}
          className="ui-btn-md border border-emerald-200 text-emerald-800 bg-white hover:bg-emerald-50"
        >
          Đăng xuất
        </button>
      </div>

      <div className="ui-card space-y-3">
        <h3 className="farm-section-title">Ghép nối thiết bị</h3>
        <button
          type="button"
          onClick={goToPairing}
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
              isAdvancedMode ? 'bg-amber-100 text-amber-800' : 'bg-emerald-100 text-emerald-800/80'
            }`}
          >
            <LockKeyhole size={16} />
          </div>
          <div>
            <p className="text-sm font-semibold text-emerald-950">Chế độ kỹ thuật</p>
            <p className="text-[11px] text-emerald-700/75">Mở rộng thông số an toàn & hiệu chuẩn</p>
          </div>
        </div>
        <Switch isOn={isAdvancedMode} onClick={onToggleAdvancedMode} colorClass="bg-amber-600" />
      </div>
    </div>
  );
};
