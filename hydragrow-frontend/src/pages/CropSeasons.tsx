import React, { useState, useEffect } from 'react';
import { useCropSeason } from '../hooks/useCropSeason';
import { Sprout, Calendar, Leaf, Play, StopCircle, CheckCircle2, History, Edit3, Save, X, FileText } from 'lucide-react';
import toast from 'react-hot-toast';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { LoadingState } from '../components/ui/LoadingState';

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
    <div className="app-page max-w-3xl mx-auto pb-32 relative">

      {/* Hiệu ứng nền Mesh Gradient */}
      <div className="absolute top-0 right-0 w-[60%] h-64 bg-gradient-to-bl from-emerald-500/10 via-transparent to-transparent pointer-events-none blur-3xl"></div>

      {/* HEADER */}
      <PageHeader
        className="animate-in slide-in-from-top-4 duration-500 mb-6"
        icon={Sprout}
        title="QUẢN LÝ MÙA VỤ"
        subtitle="Theo dõi chu kỳ sinh trưởng của cây trồng"
      />

      {/* --- PHẦN 1: MÙA VỤ ĐANG CHẠY HOẶC FORM TẠO MỚI --- */}
      <div className="relative z-10 ui-card rounded-[2rem] overflow-hidden animate-in fade-in duration-700">
        {activeSeason ? (
          <div className="p-5 md:p-6 space-y-5">
            {/* Header Thẻ Đang chạy */}
            <div className="flex items-center justify-between border-b border-slate-800/50 pb-4">
              <h2 className="text-sm font-black text-emerald-400 uppercase tracking-widest flex items-center gap-2">
                <Play size={16} className="fill-emerald-400/20" /> Mùa vụ hiện tại
              </h2>
              <div className="flex gap-2">
                {isEditing ? (
                  <button onClick={() => setIsEditing(false)} className="p-1.5 bg-slate-800 text-slate-400 rounded-lg hover:text-white transition-colors">
                    <X size={16} />
                  </button>
                ) : (
                  <button onClick={() => setIsEditing(true)} className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 hover:text-white text-[10px] font-bold uppercase transition-colors border border-slate-700">
                    <Edit3 size={12} /> Sửa
                  </button>
                )}
                <span className="px-3 py-1 bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 rounded-lg text-[10px] font-black uppercase tracking-widest flex items-center gap-1.5 shadow-[0_0_10px_rgba(16,185,129,0.2)]">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span> Đang chạy
                </span>
              </div>
            </div>

            {/* Nội dung hiển thị / Form Edit */}
            {isEditing ? (
              <div className="space-y-4 animate-in slide-in-from-left-2">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div className="space-y-1.5">
                    <label className="text-[10px] font-bold text-slate-400 uppercase tracking-widest pl-1">Tên mùa vụ</label>
                    <input type="text" value={editName} onChange={(e) => setEditName(e.target.value)} className="w-full bg-slate-950/50 border border-slate-700 rounded-xl px-4 py-2.5 text-sm text-emerald-400 font-bold focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 outline-none" />
                  </div>
                  <div className="space-y-1.5">
                    <label className="text-[10px] font-bold text-slate-400 uppercase tracking-widest pl-1">Giống cây trồng</label>
                    <input type="text" value={editPlant} onChange={(e) => setEditPlant(e.target.value)} className="w-full bg-slate-950/50 border border-slate-700 rounded-xl px-4 py-2.5 text-sm text-white focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 outline-none" />
                  </div>
                </div>
                <div className="space-y-1.5">
                  <label className="text-[10px] font-bold text-slate-400 uppercase tracking-widest pl-1">Ghi chú (Nhật ký sinh trưởng)</label>
                  <textarea rows={3} value={editDesc} onChange={(e) => setEditDesc(e.target.value)} placeholder="Ví dụ: cập nhật liều phân, thay đổi EC, ghi chú sâu bệnh..." className="w-full bg-slate-950/50 border border-slate-700 rounded-xl px-4 py-2.5 text-sm text-slate-300 focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 outline-none resize-none"></textarea>
                </div>
                <button onClick={handleUpdate} disabled={isLoading} className={`w-full flex items-center justify-center gap-2 py-3 bg-emerald-600 hover:bg-emerald-500 text-slate-950 rounded-xl shadow-[0_0_15px_rgba(16,185,129,0.4)] font-black text-xs uppercase tracking-widest transition-all ${isLoading ? 'opacity-50 cursor-not-allowed' : 'active:scale-[0.98]'}`}>
                  <Save size={16} /> {isLoading ? 'Đang lưu...' : 'Lưu thay đổi'}
                </button>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 bg-slate-950/40 p-5 rounded-2xl border border-white/5">
                <div className="space-y-1">
                  <p className="text-[10px] font-bold text-slate-500 uppercase tracking-widest">Tên mùa vụ</p>
                  <p className="text-base font-black text-emerald-400">{activeSeason.name}</p>
                </div>
                <div className="space-y-1">
                  <p className="text-[10px] font-bold text-slate-500 uppercase tracking-widest">Thời gian bắt đầu</p>
                  <p className="text-sm font-bold text-white flex items-center gap-1.5">
                    <Calendar size={14} className="text-blue-400" />
                    {new Date(activeSeason.start_time).toLocaleString('vi-VN')}
                  </p>
                </div>
                <div className="space-y-1">
                  <p className="text-[10px] font-bold text-slate-500 uppercase tracking-widest">Giống cây trồng</p>
                  <p className="text-sm font-bold text-white flex items-center gap-1.5 bg-slate-800/50 inline-flex px-3 py-1.5 rounded-lg border border-slate-700">
                    <Leaf size={14} className="text-emerald-500" />
                    {activeSeason.plant_type || 'Chưa cập nhật'}
                  </p>
                </div>
                <div className="space-y-1 md:col-span-2">
                  <p className="text-[10px] font-bold text-slate-500 uppercase tracking-widest">Ghi chú</p>
                  <div className="text-xs text-slate-400 bg-slate-900/50 p-3 rounded-lg border border-slate-800/50 flex items-start gap-2">
                    <FileText size={14} className="text-slate-500 shrink-0 mt-0.5" />
                    <p className="italic">{activeSeason.description || 'Chưa có ghi chú cho mùa vụ này.'}</p>
                  </div>
                </div>
              </div>
            )}

            {/* Nút Kết thúc Mùa vụ */}
            {!isEditing && (
              <div className="pt-2 border-t border-slate-800/50">
                <button
                  onClick={() => { if (window.confirm('Bạn có chắc muốn kết thúc mùa vụ này? Sau khi kết thúc, bạn sẽ không thể chỉnh sửa thêm.')) endSeason() }}
                  disabled={isLoading}
                  className={`w-full flex items-center justify-center gap-2 py-3 bg-rose-500/10 text-rose-500 border border-rose-500/30 rounded-xl transition-all font-black text-xs uppercase tracking-widest ${isLoading ? 'opacity-50 cursor-not-allowed' : 'hover:bg-rose-500/20 hover:shadow-[0_0_15px_rgba(244,63,94,0.3)] active:scale-[0.98]'}`}
                >
                  <StopCircle size={16} /> Kết thúc mùa vụ hiện tại
                </button>
              </div>
            )}
          </div>
        ) : (
          <form onSubmit={handleCreate} className="p-5 md:p-6 space-y-6">
            <h2 className="text-sm font-black text-cyan-400 uppercase tracking-widest flex items-center gap-2 border-b border-slate-800/50 pb-4">
              <Sprout size={18} className="text-emerald-500" /> Tạo mùa vụ mới
            </h2>
            <div className="space-y-4">
              <div className="ui-form-row">
                <label className="ui-form-label text-slate-400">Tên mùa vụ <span className="text-rose-500">*</span></label>
                <input type="text" required placeholder="VD: Dưa lưới vụ Xuân 2026" value={newName} onChange={(e) => setNewName(e.target.value)} className="ui-input bg-slate-950 border-slate-800 text-emerald-400 font-bold placeholder-slate-600 focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500" />
              </div>
              <div className="ui-form-row">
                <label className="ui-form-label text-slate-400">Giống cây trồng</label>
                <input type="text" placeholder="VD: Dưa lưới, Cà chua..." value={newPlant} onChange={(e) => setNewPlant(e.target.value)} className="ui-input bg-slate-950 border-slate-800 text-slate-50 placeholder-slate-600 focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500" />
              </div>
              <div className="ui-form-row">
                <label className="ui-form-label text-slate-400">Ghi chú ban đầu (Tùy chọn)</label>
                <textarea rows={2} placeholder="Nguồn gốc hạt giống, EC mục tiêu khởi điểm..." value={newDesc} onChange={(e) => setNewDesc(e.target.value)} className="ui-input bg-slate-950 border-slate-800 text-slate-300 placeholder-slate-600 focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 resize-none" />
              </div>
            </div>
            <button
              type="submit"
              disabled={isLoading}
              className={`w-full py-3.5 bg-gradient-to-r from-emerald-500 to-cyan-500 text-slate-950 rounded-xl shadow-[0_10px_20px_rgba(6,182,212,0.3)] font-black text-[13px] uppercase tracking-widest transition-all ${isLoading ? 'opacity-50 cursor-not-allowed' : 'hover:shadow-[0_10px_30px_rgba(6,182,212,0.5)] hover:scale-[1.02] active:scale-95'}`}
            >
              {isLoading ? 'Đang tạo...' : 'Tạo mùa vụ'}
            </button>
          </form>
        )}
      </div>

      {/* --- PHẦN 2: LỊCH SỬ MÙA VỤ --- */}
      <div className="relative z-10 ui-card rounded-[2rem] overflow-hidden animate-in fade-in slide-in-from-bottom-8 duration-700">
        <div className="p-5 border-b border-slate-800/50 bg-slate-800/20">
          <h2 className="text-sm font-black text-slate-300 uppercase tracking-widest flex items-center gap-2">
            <History size={16} className="text-indigo-400" />
            Lịch sử mùa vụ
          </h2>
        </div>

        <div className="divide-y divide-white/5">
          {filteredHistory.length === 0 ? (
            <StateView icon={History} title="Chưa có hồ sơ lưu trữ." className="opacity-70" />
          ) : (
            filteredHistory.map((season) => (
              <div key={season.id} className="p-5 hover:bg-slate-800/30 transition-colors group cursor-default">
                <div className="flex justify-between items-start mb-3">
                  <h3 className="font-bold text-slate-200 group-hover:text-emerald-400 transition-colors">{season.name}</h3>
                  {season.status === 'active' ? (
                    <span className="px-2.5 py-1 bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 rounded-lg text-[9px] font-black uppercase tracking-widest flex items-center gap-1 shadow-[0_0_10px_rgba(16,185,129,0.1)]">
                      <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span> Chạy
                    </span>
                  ) : (
                    <span className="px-2.5 py-1 bg-slate-800 text-slate-400 border border-slate-700 rounded-lg text-[9px] font-black uppercase tracking-widest flex items-center gap-1">
                      <CheckCircle2 size={12} className="text-blue-400" /> Đóng
                    </span>
                  )}
                </div>

                <div className="flex flex-col sm:flex-row sm:items-center gap-3 sm:gap-6 text-[11px] font-medium text-slate-500">
                  <span className="flex items-center gap-1.5 bg-slate-950/50 px-2.5 py-1 rounded-md border border-slate-800">
                    <Leaf size={12} className="text-emerald-500/70" />
                    {season.plant_type || 'Chưa cập nhật'}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Calendar size={12} className="text-blue-400/70" />
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
