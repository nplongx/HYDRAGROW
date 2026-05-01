import React, { useState, useEffect } from 'react';
import { useCropSeason } from '../hooks/useCropSeason';
import { Sprout, Calendar, Leaf, Play, StopCircle, CheckCircle2, History, Edit3, Save, X, FileText } from 'lucide-react';
import toast from 'react-hot-toast';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { LoadingState } from '../components/ui/LoadingState';
import { InputGroup } from '../components/ui/InputGroup';

export const CropSeasons = () => {
  const { activeSeason, history, isLoading, createSeason, endSeason, updateSeason } = useCropSeason();

  // States cho Form tạo mới
  const [newName, setNewName] = useState('');
  const [newPlant, setNewPlant] = useState('');
  const [newDesc, setNewDesc] = useState('');

  // States cho Chế độ Chỉnh sửa
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState('');
  const [editPlant, setEditPlant] = useState('');
  const [editDesc, setEditDesc] = useState('');

  // Tự động nạp dữ liệu khi bật chế độ Edit
  useEffect(() => {
    if (activeSeason && isEditing) {
      setEditName(activeSeason.name || '');
      setEditPlant(activeSeason.plant_type || '');
      setEditDesc(activeSeason.description || '');
    }
  }, [activeSeason, isEditing]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newName) return;

    const success = await createSeason(newName, newPlant, newDesc);
    if (success) {
      setNewName(''); setNewPlant(''); setNewDesc('');
      toast.success('Đã tạo mùa vụ mới.');
    }
  };

  const handleUpdate = async () => {
    if (!editName) return;
    const success = await updateSeason(editName, editPlant, editDesc);
    if (success) setIsEditing(false);
  };

  if (isLoading && !activeSeason && history.length === 0) {
    return <LoadingState message="Đang tải dữ liệu mùa vụ..." />;
  }

  const filteredHistory = history.filter(season => season.id !== activeSeason?.id);

  return (
    <div className="p-4 md:p-8 max-w-4xl mx-auto pb-28">

      {/* HEADER */}
      <PageHeader
        icon={Sprout}
        title="Quản Lý Mùa Vụ"
        subtitle="Theo dõi và ghi chép chu kỳ sinh trưởng của cây trồng"
      />

      {/* --- PHẦN 1: MÙA VỤ ĐANG CHẠY HOẶC FORM TẠO MỚI --- */}
      <div className="bg-slate-900 border border-slate-800 rounded-xl overflow-hidden mb-6">
        {activeSeason ? (
          <div className="p-5 md:p-6 flex flex-col gap-5">
            {/* Header Thẻ Đang chạy */}
            <div className="flex items-center justify-between border-b border-slate-800 pb-4">
              <div className="flex items-center gap-2 text-slate-100">
                <Play size={18} className="text-emerald-500 fill-emerald-500/20" />
                <h2 className="text-base font-semibold">Mùa vụ hiện tại</h2>
              </div>

              <div className="flex items-center gap-2">
                {isEditing ? (
                  <button onClick={() => setIsEditing(false)} className="p-1.5 bg-slate-800 text-slate-400 rounded-lg hover:text-white transition-colors">
                    <X size={16} />
                  </button>
                ) : (
                  <button onClick={() => setIsEditing(true)} className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 hover:text-white text-xs font-medium transition-colors border border-slate-700">
                    <Edit3 size={14} /> Sửa
                  </button>
                )}
                <span className="px-2.5 py-1 bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 rounded-lg text-xs font-medium flex items-center gap-1.5">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span> Đang chạy
                </span>
              </div>
            </div>

            {/* Nội dung hiển thị / Form Edit */}
            {isEditing ? (
              <div className="space-y-4 animate-in slide-in-from-left-2">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <InputGroup label="Tên mùa vụ" type="text" value={editName} onChange={(e: any) => setEditName(e.target.value)} />
                  <InputGroup label="Giống cây trồng" type="text" value={editPlant} onChange={(e: any) => setEditPlant(e.target.value)} />
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-sm font-medium text-slate-300">Ghi chú (Nhật ký sinh trưởng)</label>
                  <textarea
                    rows={3}
                    value={editDesc}
                    onChange={(e) => setEditDesc(e.target.value)}
                    placeholder="Ví dụ: cập nhật liều phân, thay đổi EC, ghi chú sâu bệnh..."
                    className="w-full bg-slate-950 border border-slate-800 text-slate-200 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-blue-500 hover:border-slate-700 resize-none transition-colors"
                  ></textarea>
                </div>
                <button
                  onClick={handleUpdate}
                  disabled={isLoading || !editName}
                  className="w-full flex items-center justify-center gap-2 py-2.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-medium text-sm transition-colors disabled:opacity-50"
                >
                  <Save size={16} /> {isLoading ? 'Đang lưu...' : 'Lưu thay đổi'}
                </button>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 bg-slate-950/50 p-4 rounded-xl border border-slate-800/50">
                <div className="space-y-1">
                  <p className="text-xs font-medium text-slate-500">Tên mùa vụ</p>
                  <p className="text-base font-semibold text-slate-100">{activeSeason.name}</p>
                </div>
                <div className="space-y-1">
                  <p className="text-xs font-medium text-slate-500">Thời gian bắt đầu</p>
                  <p className="text-sm font-medium text-slate-200 flex items-center gap-1.5">
                    <Calendar size={14} className="text-slate-400" />
                    {new Date(activeSeason.start_time).toLocaleString('vi-VN')}
                  </p>
                </div>
                <div className="space-y-1">
                  <p className="text-xs font-medium text-slate-500">Giống cây trồng</p>
                  <p className="text-sm font-medium text-slate-200 flex items-center gap-1.5">
                    <Leaf size={14} className="text-slate-400" />
                    {activeSeason.plant_type || 'Chưa cập nhật'}
                  </p>
                </div>
                <div className="space-y-1 md:col-span-2 pt-2 border-t border-slate-800/50">
                  <p className="text-xs font-medium text-slate-500 mb-1.5">Ghi chú</p>
                  <div className="text-sm text-slate-300 bg-slate-900 p-3 rounded-lg border border-slate-800 flex items-start gap-2">
                    <FileText size={16} className="text-slate-500 shrink-0 mt-0.5" />
                    <p className="leading-relaxed">{activeSeason.description || 'Chưa có ghi chú cho mùa vụ này.'}</p>
                  </div>
                </div>
              </div>
            )}

            {/* Nút Kết thúc Mùa vụ */}
            {!isEditing && (
              <div className="pt-2">
                <button
                  onClick={() => { if (window.confirm('Bạn có chắc muốn kết thúc mùa vụ này? Sau khi kết thúc, bạn sẽ không thể chỉnh sửa thêm.')) endSeason() }}
                  disabled={isLoading}
                  className="w-full flex items-center justify-center gap-2 py-2.5 bg-red-500/10 text-red-500 border border-red-500/20 rounded-lg hover:bg-red-500 hover:text-white transition-colors font-medium text-sm disabled:opacity-50"
                >
                  <StopCircle size={16} /> Kết thúc mùa vụ hiện tại
                </button>
              </div>
            )}
          </div>
        ) : (
          <form onSubmit={handleCreate} className="p-5 md:p-6 flex flex-col gap-5">
            <h2 className="text-base font-semibold text-slate-100 flex items-center gap-2 border-b border-slate-800 pb-4">
              <Sprout size={20} className="text-emerald-500" /> Tạo mùa vụ mới
            </h2>

            <div className="space-y-4">
              <InputGroup label="Tên mùa vụ *" type="text" value={newName} onChange={(e: any) => setNewName(e.target.value)} />
              <InputGroup label="Giống cây trồng" type="text" value={newPlant} onChange={(e: any) => setNewPlant(e.target.value)} />

              <div className="flex flex-col gap-1">
                <label className="text-sm font-medium text-slate-300">Ghi chú ban đầu</label>
                <textarea
                  rows={2}
                  placeholder="Nguồn gốc hạt giống, EC mục tiêu khởi điểm..."
                  value={newDesc}
                  onChange={(e) => setNewDesc(e.target.value)}
                  className="w-full bg-slate-950 border border-slate-800 text-slate-200 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-blue-500 hover:border-slate-700 resize-none transition-colors"
                />
              </div>
            </div>

            <button
              type="submit"
              disabled={isLoading || !newName}
              className="w-full py-2.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-medium text-sm transition-colors disabled:opacity-50"
            >
              {isLoading ? 'Đang tạo...' : 'Tạo mùa vụ'}
            </button>
          </form>
        )}
      </div>

      {/* --- PHẦN 2: LỊCH SỬ MÙA VỤ --- */}
      <div className="bg-slate-900 border border-slate-800 rounded-xl overflow-hidden">
        <div className="p-4 md:p-5 border-b border-slate-800 bg-slate-800/20">
          <h2 className="text-sm font-semibold text-slate-200 flex items-center gap-2">
            <History size={18} className="text-slate-400" />
            Lịch sử mùa vụ
          </h2>
        </div>

        <div className="divide-y divide-slate-800">
          {filteredHistory.length === 0 ? (
            <div className="p-8">
              <StateView icon={History} title="Chưa có hồ sơ lưu trữ." className="border-none bg-transparent" />
            </div>
          ) : (
            filteredHistory.map((season) => (
              <div key={season.id} className="p-4 md:p-5 hover:bg-slate-800/30 transition-colors">
                <div className="flex justify-between items-start mb-2">
                  <h3 className="font-medium text-slate-200">{season.name}</h3>
                  {season.status === 'active' ? (
                    <span className="px-2 py-0.5 bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 rounded text-[10px] font-medium flex items-center gap-1.5">
                      <span className="w-1 h-1 rounded-full bg-emerald-500 animate-pulse"></span> Đang chạy
                    </span>
                  ) : (
                    <span className="px-2 py-0.5 bg-slate-800 text-slate-400 border border-slate-700 rounded text-[10px] font-medium flex items-center gap-1.5">
                      <CheckCircle2 size={10} /> Đã đóng
                    </span>
                  )}
                </div>

                <div className="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4 text-xs font-medium text-slate-500">
                  <span className="flex items-center gap-1.5">
                    <Leaf size={14} className="text-slate-400" />
                    {season.plant_type || 'Chưa cập nhật'}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Calendar size={14} className="text-slate-400" />
                    {new Date(season.start_time).toLocaleDateString('vi-VN')}
                    {season.end_time ? ` → ${new Date(season.end_time).toLocaleDateString('vi-VN')}` : ' → Nay'}
                  </span>
                </div>
              </div>
            ))
          )}
        </div>
      </div>

    </div>
  );
};

export default CropSeasons;
