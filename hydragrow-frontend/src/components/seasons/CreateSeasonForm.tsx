import React, { useState } from 'react';
import { Sprout, Bookmark, CheckCircle, Droplets } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { InputGroup } from '../ui/InputGroup';
import { useDeviceStore } from '../../store/useDeviceStore';
import { httpFetch } from '../../platform/http';
import { RecipeTemplate } from '../../types/models';
import toast from 'react-hot-toast';

interface CreateSeasonFormProps {
  isLoading: boolean;
  onCreateSeason: (name: string, plantType: string, description: string, recipeId?: string) => Promise<any>;
}

export const CreateSeasonForm: React.FC<CreateSeasonFormProps> = ({ isLoading, onCreateSeason }) => {
  const settings = useDeviceStore((s) => s.settings);
  const [newName, setNewName] = useState('');
  const [selectedRecipeId, setSelectedRecipeId] = useState('');
  const [selectedRecipe, setSelectedRecipe] = useState<RecipeTemplate | null>(null);
  const [newDesc, setNewDesc] = useState('');

  const { data: recipesList = [] } = useQuery<RecipeTemplate[]>({
    queryKey: ['recipes-templates', settings?.backend_url],
    enabled: Boolean(settings?.backend_url),
    queryFn: async () => {
      const res = await httpFetch(`${settings!.backend_url}/api/recipes`, {
        headers: { 'X-API-Key': settings?.api_key || '' },
      });
      if (!res.ok) return [];
      const json = await res.json();
      return json.data || [];
    },
  });

  const handleSelectRecipe = (recipeId: string) => {
    setSelectedRecipeId(recipeId);
    const found = recipesList.find((r) => r.id === recipeId) || null;
    setSelectedRecipe(found);
    if (found && !newName) {
      setNewName(`Vụ ${found.name}`);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newName.trim()) {
      toast.error('Vui lòng nhập tên mùa vụ');
      return;
    }
    if (!selectedRecipeId || !selectedRecipe) {
      toast.error('Vui lòng chọn Công thức dinh dưỡng cho mùa vụ');
      return;
    }

    const success = await onCreateSeason(newName, selectedRecipe.crop, newDesc, selectedRecipeId);
    if (success) {
      setNewName('');
      setSelectedRecipeId('');
      setSelectedRecipe(null);
      setNewDesc('');
    }
  };

  return (
    <div className="bg-white border border-emerald-100 rounded-xl overflow-hidden mb-6 shadow-sm">
      <form onSubmit={handleSubmit} className="p-5 md:p-6 flex flex-col gap-5">
        <h2 className="text-base font-semibold text-emerald-950 flex items-center gap-2 border-b border-emerald-100 pb-4">
          <Sprout size={20} className="text-emerald-500" />
          Bắt đầu Mùa Vụ Mới
        </h2>

        <div className="space-y-4">
          {/* Chọn Công thức (Bắt buộc) */}
          <div className="flex flex-col gap-1.5">
            <label className="text-sm font-semibold text-emerald-950 flex items-center gap-2">
              <Bookmark size={15} className="text-emerald-700" />
              Công thức nuôi trồng (Recipe Template) <span className="text-red-500">*</span>
            </label>
            <select
              value={selectedRecipeId}
              onChange={(e) => handleSelectRecipe(e.target.value)}
              className="w-full bg-white border border-emerald-200 text-emerald-950 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-emerald-600 cursor-pointer"
            >
              <option value="">-- Chọn công thức chuẩn cho cây --</option>
              {recipesList.map((tmpl) => (
                <option key={tmpl.id} value={tmpl.id}>
                  {tmpl.name} (Cây trồng: {tmpl.crop}) - {tmpl.stages.length} giai đoạn
                </option>
              ))}
            </select>
          </div>

          {/* Xem trước thông số công thức */}
          {selectedRecipe && (
            <div className="rounded-xl border border-emerald-200 bg-emerald-50/70 p-3.5 space-y-2 text-xs text-emerald-900">
              <div className="flex items-center justify-between font-bold">
                <span>Giống cây: <b className="text-emerald-700 uppercase">{selectedRecipe.crop}</b></span>
                <span>Số stage: {selectedRecipe.stages.length}</span>
              </div>
              <div className="flex flex-wrap gap-2 pt-1">
                {selectedRecipe.stages.map((st, i) => (
                  <span key={i} className="inline-flex items-center gap-1 bg-white border border-emerald-200 px-2 py-1 rounded-md">
                    <Droplets size={11} className="text-blue-600" />
                    <b>{st.name}:</b> EC {st.ec_target} | pH {st.ph_target}
                  </span>
                ))}
              </div>
            </div>
          )}

          <InputGroup
            label="Tên mùa vụ"
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />

          <div className="flex flex-col gap-1">
            <label className="text-sm font-medium text-emerald-900">Ghi chú ban đầu</label>
            <textarea
              rows={2}
              placeholder="Nguồn hạt giống, thời gian gieo, mục tiêu sản lượng..."
              value={newDesc}
              onChange={(e) => setNewDesc(e.target.value)}
              className="w-full bg-white border border-emerald-200 text-emerald-950 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-emerald-600 hover:border-emerald-300 resize-none transition-colors"
            />
          </div>
        </div>

        <button
          type="submit"
          disabled={isLoading || !newName.trim() || !selectedRecipeId}
          className="w-full py-3 bg-emerald-700 hover:bg-emerald-800 text-white rounded-lg font-bold text-sm transition-colors disabled:opacity-50 flex items-center justify-center gap-2"
        >
          <CheckCircle size={16} />
          {isLoading ? 'Đang khởi tạo...' : 'Bắt đầu Mùa Vụ & Nạp Quy Trình'}
        </button>
      </form>
    </div>
  );
};