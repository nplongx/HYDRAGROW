import React, { useState, useEffect } from 'react';
import { Play, Calendar, Leaf, Edit3, Save, X, StopCircle } from 'lucide-react';
import toast from 'react-hot-toast';
import { CropSeason } from '../../types/models';
import { InputGroup } from '../ui/InputGroup';
import { ActiveRecipeStatus } from '../recipes/ActiveRecipeStatus';
import { useActiveRecipeStatus } from '../../hooks/useActiveRecipeStatus';
import { totalPlannedDays, elapsedDays, delayDays } from '../../lib/seasons/seasonProgress';

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

  const { activeRecipe, currentStage } = useActiveRecipeStatus();
  const elapsed = elapsedDays(activeSeason.start_time);
  const totalDays = activeRecipe ? totalPlannedDays(activeRecipe.stages) : null;
  const delay = activeRecipe
    ? delayDays(activeRecipe.stages, activeRecipe.current_stage_index, elapsed)
    : 0;

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
      <div className="bg-white border border-line rounded-2xl overflow-hidden shadow-sm">
        <div className="p-5 md:p-6 flex flex-col gap-5">
          {activeRecipe && totalDays !== null && (
            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs font-bold text-primary-deep">
                <span>Giai đoạn: {currentStage?.name || '—'} · Ngày {Math.floor(elapsed)}/{Math.round(totalDays)}</span>
              </div>
              <div className="h-2 bg-line rounded-full overflow-hidden">
                <div
                  className="h-full bg-primary rounded-full transition-all"
                  style={{ width: `${Math.min(100, (elapsed / totalDays) * 100)}%` }}
                />
              </div>
              {delay > 0 && (
                <div className="flex items-start gap-2 bg-[#FFFBEB] border border-amber-200 rounded-xl px-3 py-2 text-xs text-warn-deep">
                  <span className="font-bold">⚠ Chậm hơn dự kiến {Math.ceil(delay)} ngày</span>
                  <span className="text-warn-deep/80">— So với "{activeRecipe.recipe_id}" đang áp dụng</span>
                </div>
              )}
            </div>
          )}

          <div className="flex items-center justify-between border-b border-line pb-4">
            <div className="flex items-center gap-2 text-primary-deep">
              <Play size={18} className="text-primary fill-primary/20" />
              <h2 className="text-base font-bold">Mùa vụ đang chạy</h2>
            </div>
            <div className="flex items-center gap-2">
              {isEditing ? (
                <button
                  onClick={() => setIsEditing(false)}
                  className="p-1.5 bg-soft text-faint rounded-lg hover:bg-pill transition-colors"
                >
                  <X size={16} />
                </button>
              ) : (
                <button
                  onClick={() => setIsEditing(true)}
                  className="flex items-center gap-1.5 px-3 py-1.5 bg-soft text-primary-deep rounded-lg hover:bg-pill text-xs font-medium transition-colors border border-line"
                >
                  <Edit3 size={14} /> Sửa
                </button>
              )}
              <span className="px-2.5 py-1 bg-pill text-status border border-pill rounded-lg text-xs font-bold flex items-center gap-1.5">
                <span className="w-1.5 h-1.5 rounded-full bg-status animate-pulse"></span>
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
              <div className="flex flex-col gap-1">
                <label className="text-sm font-medium text-primary-deep">Ghi chú</label>
                <textarea
                  rows={3}
                  value={editDesc}
                  onChange={(e) => setEditDesc(e.target.value)}
                  className="w-full bg-white border border-line text-primary-deep text-sm rounded-lg px-3 py-2.5 outline-none focus:border-primary hover:border-line resize-none transition-colors"
                />
              </div>
              <button
                onClick={handleUpdate}
                disabled={isLoading || !editName.trim()}
                className="w-full flex items-center justify-center gap-2 py-2.5 bg-primary hover:bg-primary-deep text-white rounded-lg font-medium text-sm transition-colors disabled:opacity-50"
              >
                <Save size={16} /> {isLoading ? 'Đang lưu...' : 'Lưu thay đổi'}
              </button>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4 bg-surface-muted p-4 rounded-xl border border-line">
              <div className="space-y-1">
                <p className="text-xs font-medium text-faint">Tên mùa vụ</p>
                <p className="text-base font-bold text-primary-deep">{activeSeason.name}</p>
              </div>
              <div className="space-y-1">
                <p className="text-xs font-medium text-faint">Giống cây trồng</p>
                <p className="text-sm font-bold text-primary-deep uppercase flex items-center gap-1.5">
                  <Leaf size={14} className="text-primary" />
                  {activeSeason.plant_type || 'Chưa cập nhật'}
                </p>
              </div>
              <div className="space-y-1">
                <p className="text-xs font-medium text-faint">Thời gian bắt đầu</p>
                <p className="text-xs font-semibold text-primary-deep flex items-center gap-1.5">
                  <Calendar size={14} className="text-primary/70" />
                  {new Date(activeSeason.start_time).toLocaleString('vi-VN')}
                </p>
              </div>
              {activeSeason.description && (
                <div className="space-y-1 md:col-span-3 pt-2 border-t border-line">
                  <p className="text-xs font-medium text-faint">Ghi chú</p>
                  <p className="text-xs text-text-muted bg-white p-2.5 rounded-lg border border-line">
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
                className="w-full flex items-center justify-center gap-2 py-2.5 bg-red-50 text-red-600 border border-red-200 rounded-xl hover:bg-red-600 hover:text-white transition-colors font-bold text-xs uppercase tracking-wider disabled:opacity-50"
              >
                <StopCircle size={15} /> Kết thúc mùa vụ
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Nhúng tiến trình Recipe đang chạy ngay dưới mùa vụ */}
      <ActiveRecipeStatus />
    </div>
  );
};
