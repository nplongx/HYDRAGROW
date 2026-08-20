import React, { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  ArrowDown,
  ArrowUp,
  BookOpen,
  ClipboardList,
  Plus,
  Save,
  Trash2,
} from 'lucide-react';
import toast from 'react-hot-toast';
import { httpFetch } from '../platform/http';
import { useDeviceStore } from '../store/useDeviceStore';
import { CropStage, RecipeTemplate } from '../types/models';

type EditableStage = CropStage & {
  id: string;
  duration_days: number;
};

const createDefaultStage = (index: number): EditableStage => ({
  id: crypto.randomUUID(),
  name: `Giai đoạn ${index}`,
  duration_days: 7,
  duration_sec: 7 * 86400,
  ec_target: 1.4,
  ec_tolerance: 0.1,
  ph_target: 6.0,
  ph_tolerance: 0.2,
  nutrient_a_ratio: 1.0,
  nutrient_b_ratio: 1.0,
  water_level_target: 20.0,
  water_change_interval_days: 7,
  water_change_drain_cm: 5.0,
  misting_on_duration_ms: 10000,
  misting_off_duration_ms: 180000,
});

const toNumber = (value: string, fallback = 0) => {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
};

const RecipeBuilder: React.FC = () => {
  const queryClient = useQueryClient();
  const settings = useDeviceStore((s) => s.settings);

  const [templateName, setTemplateName] = useState('Xà lách thủy canh thương phẩm');
  const [cropType, setCropType] = useState('lettuce');
  const [description, setDescription] = useState('Quy trình chuẩn dinh dưỡng và vi khí hậu');
  const [stages, setStages] = useState<EditableStage[]>([createDefaultStage(1), createDefaultStage(2)]);
  const [selectedTemplateId, setSelectedTemplateId] = useState<string | null>(null);

  const headers = useMemo(
    () => ({
      'Content-Type': 'application/json',
      'X-API-Key': settings?.api_key || '',
    }),
    [settings?.api_key]
  );

  const timeline = useMemo(() => {
    let cursor = 1;
    return stages.map((stage, index) => {
      const startDay = cursor;
      const endDay = cursor + Math.max(1, Number(stage.duration_days || 1)) - 1;
      cursor = endDay + 1;
      return { ...stage, stageNumber: index + 1, startDay, endDay };
    });
  }, [stages]);

  const totalDays = timeline.length ? timeline[timeline.length - 1].endDay : 0;

  const { data: recipesList = [], isLoading: isLoadingTemplates } = useQuery<RecipeTemplate[]>({
    queryKey: ['recipes-templates', settings?.backend_url],
    enabled: Boolean(settings?.backend_url),
    queryFn: async () => {
      const res = await httpFetch(`${settings!.backend_url}/api/recipes`, { method: 'GET', headers });
      if (!res.ok) return [];
      const json = await res.json();
      return json.data || [];
    },
  });

  const handleSelectTemplate = (template: RecipeTemplate) => {
    setSelectedTemplateId(template.id);
    setTemplateName(template.name);
    setCropType(template.crop);
    setDescription(template.description || '');
    setStages(
      template.stages.map((stage) => ({
        ...stage,
        id: crypto.randomUUID(),
        duration_days: Math.max(1, Math.round(stage.duration_sec / 86400)),
      }))
    );
    toast.success(`Đã chọn: ${template.name}`);
  };

  const handleResetForm = () => {
    setSelectedTemplateId(null);
    setTemplateName('Công thức mới');
    setCropType('');
    setDescription('');
    setStages([createDefaultStage(1), createDefaultStage(2)]);
  };

  const saveRecipeMutation = useMutation({
    mutationFn: async () => {
      if (!settings?.backend_url) throw new Error('Thiếu backend URL.');
      const payload = {
        name: templateName.trim(),
        crop: cropType.trim().toLowerCase(),
        description,
        stages: stages.map(({ id: _id, duration_days, ...stage }) => ({
          ...stage,
          duration_sec: duration_days * 86400,
        })),
      };
      const res = await httpFetch(`${settings.backend_url}/api/recipes`, {
        method: 'POST',
        headers,
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
      return res.json();
    },
    onSuccess: () => {
      toast.success('Đã lưu Công thức mẫu vào CSDL!');
      queryClient.invalidateQueries({ queryKey: ['recipes-templates'] });
    },
    onError: (error: Error) => toast.error(`Lỗi lưu công thức: ${error.message}`),
  });

  const deleteRecipeMutation = useMutation({
    mutationFn: async (recipeId: string) => {
      if (!settings?.backend_url) throw new Error('Thiếu backend URL.');
      const res = await httpFetch(`${settings.backend_url}/api/recipes/${recipeId}`, {
        method: 'DELETE',
        headers,
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
      return res.json();
    },
    onSuccess: () => {
      toast.success('Đã xóa công thức mẫu!');
      if (selectedTemplateId) handleResetForm();
      queryClient.invalidateQueries({ queryKey: ['recipes-templates'] });
    },
    onError: (err: Error) => toast.error(`Lỗi xóa: ${err.message}`),
  });

  const handleDeleteTemplate = (e: React.MouseEvent, template: RecipeTemplate) => {
    e.stopPropagation();
    if (window.confirm(`Bạn có chắc muốn xóa công thức "${template.name}"?`)) {
      deleteRecipeMutation.mutate(template.id);
    }
  };

  const updateStage = (id: string, patch: Partial<EditableStage>) => {
    setStages((current) =>
      current.map((stage) => {
        if (stage.id !== id) return stage;
        const updated = { ...stage, ...patch };
        if (patch.duration_days !== undefined) {
          updated.duration_sec = patch.duration_days * 86400;
        }
        return updated;
      })
    );
  };

  const moveStage = (index: number, direction: -1 | 1) => {
    setStages((current) => {
      const next = [...current];
      const target = index + direction;
      if (target < 0 || target >= next.length) return current;
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  };

  return (
    <div className="app-page max-w-6xl">
      <div className="page-header">
        <div className="page-header-main">
          <div className="page-header-icon"><ClipboardList size={22} /></div>
          <div>
            <h1 className="page-header-title">Thư Viện Công Thức (Recipe Templates)</h1>
            <p className="page-header-subtitle">
              Thiết lập quy trình dinh dưỡng, tỷ lệ A:B và chu kỳ thay nước chuẩn để áp dụng khi bắt đầu mùa vụ mới.
            </p>
          </div>
        </div>
        <button
          onClick={handleResetForm}
          className="ui-btn-md bg-white border border-emerald-200 text-emerald-900 hover:bg-emerald-50 text-xs font-bold"
        >
          <Plus size={14} className="inline mr-1" /> Soạn công thức mới
        </button>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        {/* Danh sách mẫu có sẵn & Nút Xóa */}
        <div className="ui-card space-y-4 h-fit">
          <div className="flex items-center justify-between border-b border-emerald-100 pb-3">
            <h2 className="text-sm font-bold text-emerald-950 flex items-center gap-2">
              <BookOpen size={16} className="text-emerald-700" />
              Mẫu đã lưu ({recipesList.length})
            </h2>
          </div>

          {isLoadingTemplates ? (
            <p className="text-xs text-emerald-700/75 py-6 text-center">Đang tải danh sách...</p>
          ) : recipesList.length === 0 ? (
            <p className="text-xs text-emerald-700/75 py-6 text-center">Chưa có công thức mẫu nào trong CSDL.</p>
          ) : (
            <div className="space-y-2.5">
              {recipesList.map((tmpl) => {
                const isSelected = selectedTemplateId === tmpl.id;
                return (
                  <div
                    key={tmpl.id}
                    onClick={() => handleSelectTemplate(tmpl)}
                    className={`group relative w-full text-left p-3.5 rounded-xl border transition-all text-xs flex flex-col gap-1.5 cursor-pointer ${
                      isSelected
                        ? 'bg-emerald-50 border-emerald-500 shadow-sm ring-1 ring-emerald-500'
                        : 'bg-white border-emerald-100 hover:border-emerald-300'
                    }`}
                  >
                    <div className="flex items-center justify-between font-bold text-emerald-950 pr-6">
                      <span className="truncate">{tmpl.name}</span>
                      <span className="px-2 py-0.5 rounded bg-emerald-100/80 text-emerald-800 text-[10px] uppercase shrink-0">
                        {tmpl.crop}
                      </span>
                    </div>
                    {tmpl.description && (
                      <p className="text-emerald-700/80 line-clamp-1 text-[11px]">{tmpl.description}</p>
                    )}
                    <span className="text-[10px] text-emerald-600 font-medium">
                      {tmpl.stages.length} giai đoạn • {tmpl.stages.reduce((sum, s) => sum + Math.round(s.duration_sec / 86400), 0)} ngày
                    </span>

                    {/* NÚT XÓA RECIPE TEMPLATE */}
                    <button
                      title="Xóa công thức mẫu này"
                      onClick={(e) => handleDeleteTemplate(e, tmpl)}
                      className="absolute right-2.5 top-3 p-1.5 rounded-lg text-emerald-400 hover:text-red-600 hover:bg-red-50 transition-colors opacity-70 group-hover:opacity-100"
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Form soạn thảo Stages & Timeline */}
        <div className="lg:col-span-2 space-y-6">
          <section className="ui-card space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <label className="ui-form-row">
                <span className="ui-form-label">Tên công thức</span>
                <input className="ui-input" value={templateName} onChange={(e) => setTemplateName(e.target.value)} />
              </label>
              <label className="ui-form-row">
                <span className="ui-form-label">Loại cây trồng (crop)</span>
                <input
                  className="ui-input font-medium"
                  placeholder="vd: lettuce, tomato, cabbage"
                  value={cropType}
                  onChange={(e) => setCropType(e.target.value)}
                />
              </label>
            </div>
            <label className="ui-form-row">
              <span className="ui-form-label">Mô tả quy trình</span>
              <textarea
                className="ui-input min-h-16 resize-none text-xs"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
              />
            </label>
          </section>

          <section className="ui-card space-y-4">
            <div className="flex items-center justify-between gap-3 border-b border-emerald-100 pb-3">
              <h2 className="text-sm font-bold text-emerald-950">Các giai đoạn sinh trưởng (Stages)</h2>
              <button
                className="ui-btn-md bg-emerald-700 text-white hover:bg-emerald-800 py-1.5 px-3 text-xs"
                onClick={() => setStages((s) => [...s, createDefaultStage(s.length + 1)])}
              >
                <Plus size={14} className="inline mr-1" /> Thêm giai đoạn
              </button>
            </div>

            <div className="space-y-4">
              {stages.map((stage, index) => (
                <div key={stage.id} className="rounded-2xl border border-emerald-100 bg-emerald-50/50 p-4 space-y-3.5 shadow-sm">
                  <div className="flex items-center justify-between border-b border-emerald-200/60 pb-2.5">
                    <span className="font-bold text-emerald-950 text-xs">#{index + 1} • {stage.name}</span>
                    <div className="flex gap-1.5">
                      <button className="p-1.5 rounded-lg bg-white border border-emerald-200 hover:bg-emerald-100 text-emerald-900" onClick={() => moveStage(index, -1)} disabled={index === 0}><ArrowUp size={12} /></button>
                      <button className="p-1.5 rounded-lg bg-white border border-emerald-200 hover:bg-emerald-100 text-emerald-900" onClick={() => moveStage(index, 1)} disabled={index === stages.length - 1}><ArrowDown size={12} /></button>
                      <button className="p-1.5 rounded-lg bg-red-50 border border-red-200 text-red-700 hover:bg-red-100" onClick={() => setStages((s) => s.filter((item) => item.id !== stage.id))} disabled={stages.length === 1}><Trash2 size={12} /></button>
                    </div>
                  </div>

                  <div className="grid gap-3 sm:grid-cols-3">
                    <label className="ui-form-row sm:col-span-2">
                      <span className="ui-form-label">Tên giai đoạn</span>
                      <input className="ui-input" value={stage.name} onChange={(e) => updateStage(stage.id, { name: e.target.value })} />
                    </label>
                    <label className="ui-form-row">
                      <span className="ui-form-label">Thời gian (ngày)</span>
                      <input className="ui-input font-bold" type="number" min={1} value={stage.duration_days} onChange={(e) => updateStage(stage.id, { duration_days: toNumber(e.target.value, 1) })} />
                    </label>
                  </div>

                  <div className="grid gap-3 sm:grid-cols-2 md:grid-cols-4 pt-2 border-t border-emerald-100">
                    <label className="ui-form-row">
                      <span className="ui-form-label">EC mục tiêu</span>
                      <input className="ui-input" type="number" step="0.1" value={stage.ec_target} onChange={(e) => updateStage(stage.id, { ec_target: toNumber(e.target.value, 1.4) })} />
                    </label>
                    <label className="ui-form-row">
                      <span className="ui-form-label">Sai số EC (±)</span>
                      <input className="ui-input" type="number" step="0.05" value={stage.ec_tolerance} onChange={(e) => updateStage(stage.id, { ec_tolerance: toNumber(e.target.value, 0.1) })} />
                    </label>
                    <label className="ui-form-row">
                      <span className="ui-form-label">pH mục tiêu</span>
                      <input className="ui-input" type="number" step="0.1" value={stage.ph_target} onChange={(e) => updateStage(stage.id, { ph_target: toNumber(e.target.value, 6.0) })} />
                    </label>
                    <label className="ui-form-row">
                      <span className="ui-form-label">Sai số pH (±)</span>
                      <input className="ui-input" type="number" step="0.05" value={stage.ph_tolerance} onChange={(e) => updateStage(stage.id, { ph_tolerance: toNumber(e.target.value, 0.2) })} />
                    </label>
                  </div>

                  <div className="grid gap-3 sm:grid-cols-2 md:grid-cols-4 pt-2 border-t border-emerald-100">
                    <label className="ui-form-row">
                      <span className="ui-form-label">Tỷ lệ Phân A</span>
                      <input className="ui-input bg-orange-50/50 border-orange-200 font-bold" type="number" step="0.1" min={0.1} value={stage.nutrient_a_ratio} onChange={(e) => updateStage(stage.id, { nutrient_a_ratio: toNumber(e.target.value, 1.0) })} />
                    </label>
                    <label className="ui-form-row">
                      <span className="ui-form-label">Tỷ lệ Phân B</span>
                      <input className="ui-input bg-orange-50/50 border-orange-200 font-bold" type="number" step="0.1" min={0.1} value={stage.nutrient_b_ratio} onChange={(e) => updateStage(stage.id, { nutrient_b_ratio: toNumber(e.target.value, 1.0) })} />
                    </label>
                    <label className="ui-form-row">
                      <span className="ui-form-label">Chu kỳ thay nước (ngày)</span>
                      <input className="ui-input bg-blue-50/50 border-blue-200" type="number" min={1} placeholder="VD: 7" value={stage.water_change_interval_days ?? ''} onChange={(e) => updateStage(stage.id, { water_change_interval_days: e.target.value ? toNumber(e.target.value) : undefined })} />
                    </label>
                    <label className="ui-form-row">
                      <span className="ui-form-label">Mực nước mục tiêu (cm)</span>
                      <input className="ui-input bg-blue-50/50 border-blue-200" type="number" step="0.5" value={stage.water_level_target} onChange={(e) => updateStage(stage.id, { water_level_target: toNumber(e.target.value, 20) })} />
                    </label>
                  </div>
                </div>
              ))}
            </div>
          </section>

          <section className="ui-card space-y-4">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
              <div>
                <h3 className="text-sm font-bold text-emerald-950">Xem trước Lộ trình</h3>
                <p className="text-xs text-emerald-700/80 mt-0.5">Tổng thời gian chu kỳ: <b>{totalDays} ngày</b></p>
              </div>
              <button
                onClick={() => saveRecipeMutation.mutate()}
                disabled={saveRecipeMutation.isPending || !templateName.trim() || !cropType.trim()}
                className="ui-btn-md bg-emerald-700 hover:bg-emerald-800 text-white flex items-center justify-center gap-2 shadow-sm font-bold text-xs"
              >
                <Save size={15} />
                {saveRecipeMutation.isPending ? 'Đang lưu...' : 'Lưu vào Thư Viện'}
              </button>
            </div>

            <div className="space-y-2 pt-2 border-t border-emerald-100">
              {timeline.map((stage) => (
                <div key={stage.id} className="flex flex-col sm:flex-row sm:items-center justify-between rounded-xl border border-emerald-100 bg-white px-3.5 py-2.5 text-xs gap-2">
                  <div>
                    <span className="font-bold text-emerald-950">Ngày {stage.startDay} - {stage.endDay}:</span> {stage.name}
                    <span className="block text-[11px] text-emerald-700 mt-0.5">Tỷ lệ A:B: <b>{stage.nutrient_a_ratio}:{stage.nutrient_b_ratio}</b> {stage.water_change_interval_days ? `| Thay nước mỗi ${stage.water_change_interval_days} ngày` : ''}</span>
                  </div>
                  <span className="text-emerald-800 font-medium whitespace-nowrap bg-emerald-50 px-2.5 py-1 rounded-md border border-emerald-200">
                    EC {stage.ec_target} ± {stage.ec_tolerance} | pH {stage.ph_target}
                  </span>
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
};

export default RecipeBuilder;