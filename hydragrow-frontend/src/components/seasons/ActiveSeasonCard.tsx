import React, { useState, useEffect } from 'react';
import { Play, Calendar, Leaf, Edit3, Save, X, FileText, StopCircle } from 'lucide-react';
import toast from 'react-hot-toast';
import { CropSeason } from '../../types/models';
import { InputGroup } from '../ui/InputGroup';

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
      toast.error('Tên mùa vụ không được để trống.');
      return;
    }
    if (onUpdateSeason) {
      await onUpdateSeason(editName, editPlant, editDesc);
      setIsEditing(false);
    }
  };

  const handleEnd = async () => {
    if (window.confirm('Xác nhận kết thúc mùa vụ? Sau khi kết thúc, mùa vụ sẽ được đưa vào lịch sử.')) {
      await onEndSeason();
    }
  };

  return (
    <div className="bg-white border border-emerald-100 rounded-xl overflow-hidden mb-6 shadow-sm">
      <div className="p-5 md:p-6 flex flex-col gap-5">
        {/* Header Thẻ Mùa Vụ */}
        <div className="flex items-center justify-between border-b border-emerald-100 pb-4">
          <div className="flex items-center gap-2 text-emerald-950">
            <Play size={18} className="text-emerald-500 fill-emerald-500/20" />
            <h2 className="text-base font-semibold">Mùa vụ hiện tại</h2>
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
                className="flex items-center gap-1.5 px-3 py-1.5 bg-emerald-100 text-emerald-900 rounded-lg hover:bg-emerald-200 text-xs font-medium transition-colors border border-emerald-200"
              >
                <Edit3 size={14} /> Sửa
              </button>
            )}
            <span className="px-2.5 py-1 bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 rounded-lg text-xs font-medium flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
              Đang hoạt động
            </span>
          </div>
        </div>

        {/* Nội dung xem / Chỉnh sửa */}
        {isEditing ? (
          <div className="space-y-4 animate-in slide-in-from-left-2">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <InputGroup
                label="Tên mùa vụ *"
                type="text"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
              />
              <InputGroup
                label="Giống cây trồng"
                type="text"
                value={editPlant}
                onChange={(e) => setEditPlant(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-sm font-medium text-emerald-900">Ghi chú (Nhật ký canh tác)</label>
              <textarea
                rows={3}
                value={editDesc}
                onChange={(e) => setEditDesc(e.target.value)}
                placeholder="Cập nhật thông tin phân bón, mục tiêu EC/pH..."
                className="w-full bg-white border border-emerald-200 text-emerald-950 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-emerald-600 hover:border-emerald-300 resize-none transition-colors"
              />
            </div>
            <button
              onClick={handleUpdate}
              disabled={isLoading || !editName.trim()}
              className="w-full flex items-center justify-center gap-2 py-2.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-medium text-sm transition-colors disabled:opacity-50"
            >
              <Save size={16} /> {isLoading ? 'Đang lưu...' : 'Lưu thay đổi'}
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 bg-emerald-50/80 p-4 rounded-xl border border-emerald-100">
            <div className="space-y-1">
              <p className="text-xs font-medium text-emerald-700/75">Tên mùa vụ</p>
              <p className="text-base font-semibold text-emerald-950">{activeSeason.name}</p>
            </div>
            <div className="space-y-1">
              <p className="text-xs font-medium text-emerald-700/75">Thời gian bắt đầu</p>
              <p className="text-sm font-medium text-emerald-950 flex items-center gap-1.5">
                <Calendar size={14} className="text-emerald-800/80" />
                {new Date(activeSeason.start_time).toLocaleString('vi-VN')}
              </p>
            </div>
            <div className="space-y-1">
              <p className="text-xs font-medium text-emerald-700/75">Giống cây trồng</p>
              <p className="text-sm font-medium text-emerald-950 flex items-center gap-1.5">
                <Leaf size={14} className="text-emerald-800/80" />
                {activeSeason.plant_type || 'Chưa cập nhật'}
              </p>
            </div>
            <div className="space-y-1 md:col-span-2 pt-2 border-t border-emerald-100">
              <p className="text-xs font-medium text-emerald-700/75 mb-1.5">Ghi chú</p>
              <div className="text-sm text-emerald-900 bg-white p-3 rounded-lg border border-emerald-100 flex items-start gap-2">
                <FileText size={16} className="text-emerald-700/75 shrink-0 mt-0.5" />
                <p className="leading-relaxed">{activeSeason.description || 'Chưa có ghi chú nào cho mùa vụ này.'}</p>
              </div>
            </div>
          </div>
        )}

        {!isEditing && (
          <div className="pt-2">
            <button
              onClick={handleEnd}
              disabled={isLoading}
              className="w-full flex items-center justify-center gap-2 py-2.5 bg-red-50 text-red-600 border border-red-200 rounded-lg hover:bg-red-600 hover:text-white transition-colors font-medium text-sm disabled:opacity-50"
            >
              <StopCircle size={16} /> Kết thúc mùa vụ
            </button>
          </div>
        )}
      </div>
    </div>
  );
};
