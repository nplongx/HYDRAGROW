import React, { useMemo, useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { ArrowDown, ArrowUp, Bookmark, ClipboardList, Plus, Send, Trash2, XCircle } from 'lucide-react';
import toast from 'react-hot-toast';
import { ActiveRecipeStatus } from '../components/recipes/ActiveRecipeStatus';
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
  const settings = useDeviceStore((s) => s.settings);
  const deviceId = useDeviceStore((s) => s.deviceId);
  const [templateName, setTemplateName] = useState('Xà lách thủy canh thương phẩm');
  const [cropType, setCropType] = useState('lettuce');
  const [description, setDescription] = useState('Quy trình chuẩn 30 ngày cho xà lách mỡ xoăn');
  const [stages, setStages] = useState<EditableStage[]>([createDefaultStage(1), createDefaultStage(2)]);
  const [selectedRecipeId, setSelectedRecipeId] = useState<string | null>(null);

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

  const { data: recipesList = [], refetch: refetchRecipes } = useQuery<RecipeTemplate[]>({
    queryKey: ['recipes-templates', settings?.backend_url],
    enabled: Boolean(settings?.backend_url),
    queryFn: async () => {
      const res = await httpFetch(`${settings!.backend_url}/api/recipes`, { method: 'GET', headers });
      if (!res.ok) return [];
      const json = await res.json();
      return json.data || [];
    },
  });

  const handleSelectExistingTemplate = (templateId: string) => {
    if (!templateId) {
      setSelectedRecipeId(null);
      return;
    }
    const found = recipesList.find((r) => r.id === templateId);
    if (!found) return;

    setSelectedRecipeId(found.id);
    setTemplateName(found.name);
    setCropType(found.crop);
    setDescription(found.description || '');
    setStages(
      found.stages.map((stage) => ({
        ...stage,
        id: crypto.randomUUID(),
        duration_days: Math.max(1, Math.round(stage.duration_sec / 86400)),
      }))
    );
    toast.success(`Đã nạp mẫu: ${found.name}`);
  };

  const createRecipeMutation = useMutation({
    mutationFn: async () => {
      if (!settings?.backend_url) throw new Error('Thiếu backend URL.');
      const payload = {
        name: templateName,
        crop: cropType,
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
    onSuccess: (res) => {
      const newId = res.data?.id || res.id;
      setSelectedRecipeId(newId);
      toast.success('Đã tạo Recipe Template thành công.');
      refetchRecipes();
    },
    onError: (error: Error) => toast.error(`Lỗi tạo recipe: ${error.message}`),
  });

  const applyRecipeMutation = useMutation({
    mutationFn: async () => {
      if (!settings?.backend_url || !deviceId) throw new Error('Thiếu Backend URL hoặc Device ID.');
      const res = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/recipe/apply`, {
        method: 'POST',
        headers,
        body: JSON.stringify(
          selectedRecipeId
            ? { recipe_id: selectedRecipeId }
            : {
                recipe: {
                  name: templateName,
                  crop: cropType,
                  description,
                  stages: stages.map(({ id: _id, duration_days, ...stage }) => ({
                    ...stage,
                    duration_sec: duration_days * 86400,
                  })),
                },
              }
        ),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
      return res.json();
    },
    onSuccess: () => {
      toast.success('Đã áp dụng Recipe xuống thiết bị thành công!');
    },
    onError: (error: Error) => toast.error(`Lỗi áp dụng recipe: ${error.message}`),
  });

  const clearRecipeMutation = useMutation({
    mutationFn: async () => {
      if (!settings?.backend_url || !deviceId) throw new Error('Thiếu cấu hình kết nối.');
      const res = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/recipe/clear`, {
        method: 'POST',
        headers,
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return res.json();
    },
    onSuccess: () => toast.success('Đã hủy Recipe trên thiết bị!'),
    onError: (err: Error) => toast.error(`Lỗi hủy recipe: ${err.message}`),
  });

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
    <div className="app-page max-w-5xl">
      <div className="page-header">
        <div className="page-header-main">
          <div className="page-header-icon"><ClipboardList size={22} /></div>
          <div>
            <h1 className="page-header-title">Thiết Kế Quy Trình (Recipe Builder)</h1>
            <p className="page-header-subtitle">
              Quản lý tỷ lệ dinh dưỡng A:B, chu kỳ thay nước và mục tiêu vi khí hậu theo từng độ tuổi cây.
            </p>
          </div>
        </div>
      </div>

      <section className="ui-card space-y-4">
        {recipesList.length > 0 && (
          <div className="rounded-xl border border-emerald-200/80 bg-emerald-50/60 p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div className="flex items-center gap-2 text-xs font-semibold text-emerald-950">
              <Bookmark size={16} className="text-emerald-700" />
              <span>Nạp nhanh từ mẫu đã lưu:</span>
            </div>
            <select
              value={selectedRecipeId || ''}
              onChange={(e) => handleSelectExistingTemplate(e.target.value)}
              className="bg-white border border-emerald-200 rounded-lg px-3 py-1.5 text-xs text-emerald-950 font-medium outline-none focus:border-emerald-600 cursor-pointer"
            >
              <option value="">-- Chọn công thức có sẵn --</option>
              {recipesList.map((tmpl) => (
                <option key={tmpl.id} value={tmpl.id}>
                  {tmpl.name} ({tmpl.crop})
                </option>
              ))}
            </select>
          </div>
        )}

        <div className="grid gap-4 md:grid-cols-3">
          <label className="ui-form-row">
            <span className="ui-form-label">Tên quy trình</span>
            <input className="ui-input" value={templateName} onChange={(e) => setTemplateName(e.target.value)} />
          </label>
          <label className="ui-form-row">
            <span className="ui-form-label">Loại cây trồng (crop)</span>
            <input className="ui-input" value={cropType} onChange={(e) => setCropType(e.target.value)} />
          </label>
          <label className="ui-form-row">
            <span className="ui-form-label">Áp dụng cho trạm</span>
            <input className="ui-input font-mono font-bold bg-emerald-50 text-emerald-900" value={deviceId || 'Chưa chọn'} disabled />
          </label>
        </div>
        <label className="ui-form-row">
          <span className="ui-form-label">Mô tả / Ghi chú</span>
          <textarea className="ui-input min-h-20 resize-none" value={description} onChange={(e) => setDescription(e.target.value)} />
        </label>
      </section>

      <section className="ui-card space-y-4">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-lg font-bold text-emerald-950">Danh sách Giai đoạn (Stages)</h2>
          <button className="ui-btn-md bg-emerald-700 text-white hover:bg-emerald-800" onClick={() => setStages((s) => [...s, createDefaultStage(s.length + 1)])}>
            <Plus size={16} className="inline mr-1" /> Thêm giai đoạn
          </button>
        </div>

        <div className="space-y-4">
          {stages.map((stage, index) => (
            <div key={stage.id} className="rounded-2xl border border-emerald-100 bg-emerald-50/50 p-4 md:p-5 space-y-4 shadow-sm">
              <div className="flex flex-wrap items-center justify-between gap-2 border-b border-emerald-200/60 pb-3">
                <span className="font-bold text-emerald-900 text-sm">Giai đoạn #{index + 1}: {stage.name}</span>
                <div className="flex gap-2">
                  <button className="ui-btn-md bg-white border border-emerald-200 py-1 px-3 text-xs" onClick={() => moveStage(index, -1)} disabled={index === 0}><ArrowUp size={14} /></button>
                  <button className="ui-btn-md bg-white border border-emerald-200 py-1 px-3 text-xs" onClick={() => moveStage(index, 1)} disabled={index === stages.length - 1}><ArrowDown size={14} /></button>
                  <button className="ui-btn-md bg-red-50 text-red-700 border border-red-200 py-1 px-3 text-xs" onClick={() => setStages((s) => s.filter((item) => item.id !== stage.id))} disabled={stages.length === 1}><Trash2 size={14} /></button>
                </div>
              </div>

              <div className="grid gap-3 sm:grid-cols-2 md:grid-cols-4">
                <label className="ui-form-row md:col-span-2">
                  <span className="ui-form-label">Tên giai đoạn</span>
                  <input className="ui-input" value={stage.name} onChange={(e) => updateStage(stage.id, { name: e.target.value })} />
                </label>
                <label className="ui-form-row">
                  <span className="ui-form-label">Thời gian (ngày)</span>
                  <input className="ui-input" type="number" min={1} value={stage.duration_days} onChange={(e) => updateStage(stage.id, { duration_days: toNumber(e.target.value, 1) })} />
                </label>
                <label className="ui-form-row">
                  <span className="ui-form-label">Mực nước mục tiêu (cm)</span>
                  <input className="ui-input" type="number" step="0.5" value={stage.water_level_target} onChange={(e) => updateStage(stage.id, { water_level_target: toNumber(e.target.value, 20) })} />
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
                  <input className="ui-input bg-orange-50/50 border-orange-200" type="number" step="0.1" min={0.1} value={stage.nutrient_a_ratio} onChange={(e) => updateStage(stage.id, { nutrient_a_ratio: toNumber(e.target.value, 1.0) })} />
                </label>
                <label className="ui-form-row">
                  <span className="ui-form-label">Tỷ lệ Phân B</span>
                  <input className="ui-input bg-orange-50/50 border-orange-200" type="number" step="0.1" min={0.1} value={stage.nutrient_b_ratio} onChange={(e) => updateStage(stage.id, { nutrient_b_ratio: toNumber(e.target.value, 1.0) })} />
                </label>
                <label className="ui-form-row">
                  <span className="ui-form-label">Chu kỳ thay nước (ngày)</span>
                  <input className="ui-input bg-blue-50/50 border-blue-200" type="number" min={1} placeholder="VD: 7" value={stage.water_change_interval_days ?? ''} onChange={(e) => updateStage(stage.id, { water_change_interval_days: e.target.value ? toNumber(e.target.value) : undefined })} />
                </label>
                <label className="ui-form-row">
                  <span className="ui-form-label">Xả thay nước (cm)</span>
                  <input className="ui-input bg-blue-50/50 border-blue-200" type="number" step="0.5" placeholder="VD: 5.0" value={stage.water_change_drain_cm ?? ''} onChange={(e) => updateStage(stage.id, { water_change_drain_cm: e.target.value ? toNumber(e.target.value) : undefined })} />
                </label>
              </div>

              <div className="grid gap-3 sm:grid-cols-2 md:grid-cols-3 pt-2 border-t border-emerald-100">
                <label className="ui-form-row">
                  <span className="ui-form-label">Phun sương Bật (ms)</span>
                  <input className="ui-input" type="number" step="1000" value={stage.misting_on_duration_ms} onChange={(e) => updateStage(stage.id, { misting_on_duration_ms: toNumber(e.target.value, 10000) })} />
                </label>
                <label className="ui-form-row">
                  <span className="ui-form-label">Phun sương Nghỉ (ms)</span>
                  <input className="ui-input" type="number" step="1000" value={stage.misting_off_duration_ms} onChange={(e) => updateStage(stage.id, { misting_off_duration_ms: toNumber(e.target.value, 180000) })} />
                </label>
                <label className="ui-form-row">
                  <span className="ui-form-label">Max châm 1 lần (ml)</span>
                  <input className="ui-input" type="number" placeholder="Theo an toàn mặc định" value={stage.max_dose_per_cycle_ml ?? ''} onChange={(e) => updateStage(stage.id, { max_dose_per_cycle_ml: e.target.value ? toNumber(e.target.value) : undefined })} />
                </label>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="grid gap-4 lg:grid-cols-2">
        <div className="ui-card space-y-3">
          <h2 className="text-lg font-bold text-emerald-950">Xem trước Lộ trình (Timeline)</h2>
          <p className="text-sm">Tổng thời gian toàn bộ chu kỳ: <b className="text-emerald-800">{totalDays}</b> ngày.</p>
          <div className="space-y-2">
            {timeline.map((stage) => (
              <div key={stage.id} className="flex flex-col sm:flex-row sm:items-center justify-between rounded-xl border border-emerald-100 bg-white px-4 py-3 text-sm gap-2">
                <div>
                  <span className="font-bold text-emerald-950">Ngày {stage.startDay} - {stage.endDay}:</span> {stage.name}
                  <span className="block text-[11px] text-emerald-700">Tỷ lệ A:B: <b>{stage.nutrient_a_ratio}:{stage.nutrient_b_ratio}</b> {stage.water_change_interval_days ? `| Thay nước mỗi ${stage.water_change_interval_days} ngày` : ''}</span>
                </div>
                <span className="text-emerald-800 font-medium whitespace-nowrap text-xs bg-emerald-50 px-2.5 py-1 rounded-md border border-emerald-200">
                  EC {stage.ec_target} ± {stage.ec_tolerance} | pH {stage.ph_target}
                </span>
              </div>
            ))}
          </div>
        </div>

        <div className="ui-card space-y-4">
          <ActiveRecipeStatus />
          <div className="flex flex-col gap-3">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <button className="ui-btn-md bg-emerald-700 text-white hover:bg-emerald-800 flex items-center justify-center gap-2" onClick={() => createRecipeMutation.mutate()} disabled={createRecipeMutation.isPending || !templateName.trim()}>
                <Send size={16} /> Lưu thành Template
              </button>
              <button className="ui-btn-md bg-blue-600 text-white hover:bg-blue-700 flex items-center justify-center gap-2" onClick={() => applyRecipeMutation.mutate()} disabled={applyRecipeMutation.isPending || !deviceId}>
                <ClipboardList size={16} /> Nạp vào Trạm ESP32
              </button>
            </div>
            <button className="ui-btn-md bg-red-50 text-red-700 border border-red-200 hover:bg-red-100 flex items-center justify-center gap-2" onClick={() => clearRecipeMutation.mutate()} disabled={clearRecipeMutation.isPending || !deviceId}>
              <XCircle size={16} /> Xóa Recipe trên Trạm
            </button>
          </div>
        </div>
      </section>
    </div>
  );
};

export default RecipeBuilder;