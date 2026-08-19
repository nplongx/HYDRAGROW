import React, { useMemo, useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { ArrowDown, ArrowUp, ClipboardList, Plus, Send, Trash2 } from 'lucide-react';
import toast from 'react-hot-toast';
import { ActiveRecipeStatus } from '../components/recipes/ActiveRecipeStatus';
import { httpFetch } from '../platform/http';
import { useDeviceStore } from '../store/useDeviceStore';

type RecipeStage = {
  id: string;
  name: string;
  duration_days: number;
  ec: number;
  ph: number;
  misting_on_ms: number;
  misting_off_ms: number;
};

type CreatedRecipe = {
  id?: string;
  recipe_id?: string;
  data?: { id?: string; recipe_id?: string };
  [key: string]: any;
};

const createStage = (index: number): RecipeStage => ({
  id: crypto.randomUUID(),
  name: `Stage ${index}`,
  duration_days: 7,
  ec: 1.2,
  ph: 6,
  misting_on_ms: 10000,
  misting_off_ms: 180000,
});

const getRecipeId = (recipe: CreatedRecipe | null) =>
  recipe?.id || recipe?.recipe_id || recipe?.data?.id || recipe?.data?.recipe_id || '';

const toNumber = (value: string, fallback = 0) => {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
};

const RecipeBuilder: React.FC = () => {
  const settings = useDeviceStore((s) => s.settings);
  const deviceId = useDeviceStore((s) => s.deviceId);
  const [templateName, setTemplateName] = useState('HydraGrow Template');
  const [description, setDescription] = useState('');
  const [stages, setStages] = useState<RecipeStage[]>([createStage(1), createStage(2)]);
  const [createdRecipe, setCreatedRecipe] = useState<CreatedRecipe | null>(null);

  const headers = useMemo(() => ({
    'Content-Type': 'application/json',
    'X-API-Key': settings?.api_key || '',
  }), [settings?.api_key]);

  const timeline = useMemo(() => {
    let cursor = 1;
    return stages.map((stage, index) => {
      const startDay = cursor;
      const endDay = cursor + Math.max(0, Number(stage.duration_days || 0)) - 1;
      cursor = endDay + 1;
      return { ...stage, stageNumber: index + 1, startDay, endDay };
    });
  }, [stages]);

  const totalDays = timeline.length ? timeline[timeline.length - 1].endDay : 0;

  const recipeStatus = useQuery({
    queryKey: ['recipe-status', settings?.backend_url, deviceId],
    enabled: Boolean(settings?.backend_url && deviceId),
    queryFn: async () => {
      const res = await httpFetch(`${settings!.backend_url}/api/devices/${deviceId}/recipe/status`, { method: 'GET', headers });
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
      return res.json();
    },
  });

  const createRecipe = useMutation({
    mutationFn: async () => {
      if (!settings?.backend_url) throw new Error('Thiếu backend URL.');
      const payload = {
        name: templateName,
        description,
        stages: stages.map(({ id: _id, ...stage }, index) => ({ ...stage, stage_order: index + 1 })),
      };
      const res = await httpFetch(`${settings.backend_url}/api/recipes`, {
        method: 'POST',
        headers,
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
      return res.json();
    },
    onSuccess: (recipe) => {
      setCreatedRecipe(recipe);
      toast.success('Đã tạo recipe template.');
    },
    onError: (error: Error) => toast.error(`Không thể tạo recipe: ${error.message}`),
  });

  const applyRecipe = useMutation({
    mutationFn: async () => {
      if (!settings?.backend_url || !deviceId) throw new Error('Thiếu backend URL hoặc Device ID.');
      const recipeId = getRecipeId(createdRecipe);
      const res = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/recipe/apply`, {
        method: 'POST',
        headers,
        body: JSON.stringify(recipeId ? { recipe_id: recipeId } : { recipe: createdRecipe }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
      return res.json();
    },
    onSuccess: () => {
      toast.success('Đã áp dụng recipe cho thiết bị.');
      recipeStatus.refetch();
    },
    onError: (error: Error) => toast.error(`Không thể áp dụng recipe: ${error.message}`),
  });

  const updateStage = (id: string, patch: Partial<RecipeStage>) => {
    setStages((current) => current.map((stage) => stage.id === id ? { ...stage, ...patch } : stage));
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
    <div className="app-page">
      <div className="page-header">
        <div className="page-header-main">
          <div className="page-header-icon"><ClipboardList size={22} /></div>
          <div>
            <h1 className="page-header-title">Recipe Builder</h1>
            <p className="page-header-subtitle">Tạo template dinh dưỡng theo stage, xem timeline và áp dụng recipe cho thiết bị hiện tại.</p>
          </div>
        </div>
      </div>

      <section className="ui-card space-y-4">
        <div className="grid gap-4 md:grid-cols-2">
          <label className="ui-form-row">
            <span className="ui-form-label">Tên template</span>
            <input className="ui-input" value={templateName} onChange={(e) => setTemplateName(e.target.value)} />
          </label>
          <label className="ui-form-row">
            <span className="ui-form-label">Device ID áp dụng</span>
            <input className="ui-input" value={deviceId || ''} disabled placeholder="Chưa cấu hình Device ID" />
          </label>
        </div>
        <label className="ui-form-row">
          <span className="ui-form-label">Mô tả</span>
          <textarea className="ui-input min-h-24" value={description} onChange={(e) => setDescription(e.target.value)} placeholder="Ghi chú cây trồng, giống, hoặc mục tiêu mùa vụ" />
        </label>
      </section>

      <section className="ui-card space-y-4">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-lg font-bold text-emerald-950">Stages</h2>
          <button className="ui-btn-md bg-emerald-700 text-white hover:bg-emerald-800" onClick={() => setStages((s) => [...s, createStage(s.length + 1)])}>
            <Plus size={16} className="inline mr-1" /> Thêm stage
          </button>
        </div>

        <div className="space-y-3">
          {stages.map((stage, index) => (
            <div key={stage.id} className="rounded-2xl border border-emerald-100 bg-emerald-50/60 p-4 space-y-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <strong>#{index + 1}</strong>
                <div className="flex gap-2">
                  <button className="ui-btn-md bg-white border border-emerald-200" onClick={() => moveStage(index, -1)} disabled={index === 0}><ArrowUp size={16} /></button>
                  <button className="ui-btn-md bg-white border border-emerald-200" onClick={() => moveStage(index, 1)} disabled={index === stages.length - 1}><ArrowDown size={16} /></button>
                  <button className="ui-btn-md bg-red-50 text-red-700 border border-red-100" onClick={() => setStages((s) => s.filter((item) => item.id !== stage.id))} disabled={stages.length === 1}><Trash2 size={16} /></button>
                </div>
              </div>
              <div className="grid gap-3 md:grid-cols-6">
                <label className="ui-form-row md:col-span-2"><span className="ui-form-label">Tên stage</span><input className="ui-input" value={stage.name} onChange={(e) => updateStage(stage.id, { name: e.target.value })} /></label>
                <label className="ui-form-row"><span className="ui-form-label">duration_days</span><input className="ui-input" type="number" min={1} value={stage.duration_days} onChange={(e) => updateStage(stage.id, { duration_days: toNumber(e.target.value, 1) })} /></label>
                <label className="ui-form-row"><span className="ui-form-label">EC</span><input className="ui-input" type="number" step="0.1" value={stage.ec} onChange={(e) => updateStage(stage.id, { ec: toNumber(e.target.value) })} /></label>
                <label className="ui-form-row"><span className="ui-form-label">pH</span><input className="ui-input" type="number" step="0.1" value={stage.ph} onChange={(e) => updateStage(stage.id, { ph: toNumber(e.target.value) })} /></label>
                <label className="ui-form-row"><span className="ui-form-label">Misting on/off (ms)</span><div className="grid grid-cols-2 gap-2"><input className="ui-input" type="number" min={0} value={stage.misting_on_ms} onChange={(e) => updateStage(stage.id, { misting_on_ms: toNumber(e.target.value) })} /><input className="ui-input" type="number" min={0} value={stage.misting_off_ms} onChange={(e) => updateStage(stage.id, { misting_off_ms: toNumber(e.target.value) })} /></div></label>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="grid gap-4 lg:grid-cols-2">
        <div className="ui-card space-y-3">
          <h2 className="text-lg font-bold text-emerald-950">Preview timeline</h2>
          <p className="text-sm">Tổng thời gian: <b>{totalDays}</b> ngày.</p>
          <div className="space-y-2">
            {timeline.map((stage) => (
              <div key={stage.id} className="flex items-center justify-between rounded-xl border border-emerald-100 bg-white px-4 py-3 text-sm">
                <span><b>Ngày {stage.startDay}</b> → {stage.endDay}: {stage.name}</span>
                <span className="text-emerald-700">EC {stage.ec} · pH {stage.ph}</span>
              </div>
            ))}
          </div>
        </div>

        <div className="ui-card space-y-4">
          <ActiveRecipeStatus />
          <div className="grid gap-3 md:grid-cols-2">
            <button className="ui-btn-md bg-emerald-700 text-white hover:bg-emerald-800" onClick={() => createRecipe.mutate()} disabled={createRecipe.isPending || !templateName.trim()}>
              <Send size={16} className="inline mr-1" /> Tạo recipe
            </button>
            <button className="ui-btn-md bg-amber-600 text-white hover:bg-amber-700" onClick={() => applyRecipe.mutate()} disabled={applyRecipe.isPending || !createdRecipe}>
              Áp dụng cho device
            </button>
          </div>
          {createdRecipe && <p className="ui-helper-text">Recipe vừa tạo: {getRecipeId(createdRecipe) || 'backend không trả về id; sẽ gửi lại payload khi apply.'}</p>}
        </div>
      </section>
    </div>
  );
};

export default RecipeBuilder;
