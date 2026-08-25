import React, { useState, useEffect } from 'react';
import { Play, Calendar, Leaf, Edit3, Save, X, StopCircle } from 'lucide-react';
import toast from 'react-hot-toast';
import { CropSeason } from '../../types/models';
import { InputGroup } from '../ui/InputGroup';
import { ActiveRecipeStatus } from '../recipes/ActiveRecipeStatus';

interface ActiveSeasonCardProps {
  activeSeason: CropSeason;
  isLoading: boolean;
  onEndSeason: () => Promise<any>;
  onUpdateSeason?: (name: string, plantType: string, description: string) => Promise<any>;
}

export const ActiveSeasonCard: React.FC<ActiveSeasonCardProps> = ({
  activeSeason,
  isLoading,
  onEndSeason,
  onUpdateSeason,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(activeSeason.name || '');
  const [editPlant, setEditPlant] = useState(activeSeason.plant_type || '');
  const [editDesc, setEditDesc] = useState(activeSeason.description || '');

  useEffect(() => {
    if (activeSeason && isEditing) {
      setEditName(activeSeason.name || '');
      setEditPlant(activeSeason.plant_type || '');
      setEditDesc(activeSeason.description || '');
    }
  }, [activeSeason, isEditing]);

  const handleUpdate = async () => {
    if (!editName.trim()) {
      toast.error('Tên không được để trống.');
      return;
    }
    if (onUpdateSeason) {
      await onUpdateSeason(editName, editPlant, editDesc);
      setIsEditing(false);
    }
  };

  const handleEnd = async () => {
    if (window.confirm('Xác nhận kết thúc mùa vụ? Sau khi kết thúc, quy trình nuôi trồng trên trạm sẽ được hoàn tất và chuyển vào lịch sử.')) {
      await onEndSeason();
    }
  };

  return (
    <div className="space-y-6 mb-6">
      <div className="ui-card space-y-5">
        <div className="flex items-center justify-between border-b border-emerald-100 pb-4">
          <div className="flex items-center gap-2 text-emerald-950">
            <Play size={18} className="text-emerald-500 fill-emerald-500/20" />
            <h2 className="text-base font-bold">Mùa vụ đang chạy</h2>
          </div>
          <div className="flex items-center gap-2">
            {isEditing ? (
              <button
                onClick={() => setIsEditing(false)}
                className="p-1.5 bg-emerald-100 text-emerald-800 rounded-lg hover:bg-emerald-200 transition-colors"
              >
                <X size={16} />
              </button>
            ) : (
              <button
                onClick={() => setIsEditing(true)}
                className="ui-btn-secondary text-xs py-1.5 px-3"
              >
                <Edit3 size={14} /> Sửa
              </button>
            )}
            <span className="bg-emerald-100 text-emerald-700 text-xs font-bold px-2 py-0.5 rounded-full flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
              Đang hoạt động
            </span>
          </div>
        </div>

        {isEditing ? (
          <div className="space-y-4 animate-in slide-in-from-left-2">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <InputGroup
                label="Tên mùa vụ"
                type="text"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
              />
              <InputGroup
                label="Giống cây trồng (Khóa theo Recipe)"
                type="text"
                value={editPlant}
                onChange={(e) => setEditPlant(e.target.value)}
              />
            </div>
            <div className="ui-form-row">
              <label className="ui-form-label">Ghi chú</label>
              <textarea
                rows={3}
                value={editDesc}
                onChange={(e) => setEditDesc(e.target.value)}
                className="ui-input resize-none"
              />
            </div>
            <button
              onClick={handleUpdate}
              disabled={isLoading || !editName.trim()}
              className="ui-btn-primary w-full"
            >
              <Save size={16} /> {isLoading ? 'Đang lưu...' : 'Lưu thay đổi'}
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 bg-emerald-50/80 p-4 rounded-xl border border-emerald-100">
            <div className="space-y-1">
              <p className="text-xs font-medium text-emerald-700/75">Tên mùa vụ</p>
              <p className="text-lg font-bold text-emerald-950">{activeSeason.name}</p>
            </div>
            <div className="space-y-1">
              <p className="text-xs font-medium text-emerald-700/75">Giống cây trồng</p>
              <p className="text-sm font-bold text-emerald-800 uppercase flex items-center gap-1.5">
                <Leaf size={14} className="text-emerald-700" />
                {activeSeason.plant_type || 'Chưa cập nhật'}
              </p>
            </div>
            <div className="space-y-1">
              <p className="text-xs font-medium text-emerald-700/75">Thời gian bắt đầu</p>
              <p className="text-sm text-emerald-700/60 font-semibold flex items-center gap-1.5">
                <Calendar size={14} className="text-emerald-800/80" />
                {new Date(activeSeason.start_time).toLocaleString('vi-VN')}
              </p>
            </div>
            {activeSeason.description && (
              <div className="space-y-1 md:col-span-3 pt-2 border-t border-emerald-100">
                <p className="text-xs font-medium text-emerald-700/75">Ghi chú</p>
                <p className="text-xs text-emerald-900 bg-white p-2.5 rounded-lg border border-emerald-100">
                  {activeSeason.description}
                </p>
              </div>
            )}
          </div>
        )}

        {!isEditing && (
          <div>
            <button
              onClick={handleEnd}
              disabled={isLoading}
              className="ui-btn-danger text-xs w-full uppercase tracking-wider"
            >
              <StopCircle size={15} /> Kết thúc mùa vụ
            </button>
          </div>
        )}
      </div>

      {/* Nhúng tiến trình Recipe đang chạy ngay dưới mùa vụ */}
      <ActiveRecipeStatus />
    </div>
  );
};
