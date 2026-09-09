import React from 'react';
import { RefreshCcw, Sprout, Droplets, Waves, CheckCircle2 } from 'lucide-react';
import { useActiveRecipeStatus } from '../../hooks/useActiveRecipeStatus';

export const ActiveRecipeStatus: React.FC = () => {
  const { activeRecipe, currentStage, isError, error, isFetching, refetch } = useActiveRecipeStatus();

  return (
    <section className="ui-card space-y-3">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-bold text-emerald-950 flex items-center gap-2">
            <Sprout size={18} className="text-emerald-700" />
            Quy trình đang chạy (Active Recipe)
          </h2>
          <p className="text-xs text-emerald-800/75">Trạng thái đồng bộ thời gian thực từ FSM Controller Node</p>
        </div>
        <button
          className="ui-btn-md bg-white border border-emerald-200 py-1.5 px-3 text-xs text-emerald-900 flex items-center gap-1.5"
          onClick={() => refetch()}
          disabled={isFetching}
        >
          <RefreshCcw size={14} className={isFetching ? 'animate-spin' : ''} />
          Làm mới
        </button>
      </div>

      {isError ? (
        <div className="rounded-xl border border-red-200 bg-red-50 p-3 text-xs text-red-700">
          Không thể lấy thông tin recipe: {(error as Error).message}
        </div>
      ) : !activeRecipe ? (
        <div className="rounded-xl border border-emerald-200/70 bg-emerald-50/50 p-4 text-center text-xs text-emerald-800">
          Trạm đang hoạt động theo ngưỡng cấu hình mặc định (Chưa nạp Recipe tự động).
        </div>
      ) : (
        <div className="space-y-3">
          <div className="grid gap-3 sm:grid-cols-3">
            <div className="farm-muted-panel">
              <span className="ui-form-label">Mã Recipe / Rev</span>
              <p className="font-bold text-emerald-950 text-sm truncate font-mono">
                {activeRecipe.recipe_id} (r{activeRecipe.revision})
              </p>
            </div>
            <div className="farm-muted-panel">
              <span className="ui-form-label">Giai đoạn hiện tại</span>
              <p className="font-bold text-emerald-950 text-sm flex items-center gap-1.5">
                <CheckCircle2 size={14} className="text-emerald-600" />
                {currentStage?.name || `Stage #${activeRecipe.current_stage_index + 1}`}
              </p>
            </div>
            <div className="farm-muted-panel">
              <span className="ui-form-label">Tỷ lệ Dinh dưỡng A:B</span>
              <p className="font-bold text-orange-700 text-sm font-mono">
                {currentStage ? `${currentStage.nutrient_a_ratio} : ${currentStage.nutrient_b_ratio}` : '1.0 : 1.0'}
              </p>
            </div>
          </div>

          {currentStage && (
            <div className="bg-white border border-emerald-100 rounded-xl p-3.5 flex flex-wrap items-center justify-between gap-3 text-xs">
              <div className="flex items-center gap-2">
                <Droplets size={14} className="text-sky-600" />
                <span>EC: <b>{currentStage.ec_target} ± {currentStage.ec_tolerance}</b></span>
                <span className="text-emerald-300">|</span>
                <span>pH: <b>{currentStage.ph_target} ± {currentStage.ph_tolerance}</b></span>
              </div>
              <div className="flex items-center gap-2 text-emerald-800">
                <Waves size={14} className="text-sky-600" />
                <span>Mực nước: <b>{currentStage.water_level_target} cm</b></span>
                {currentStage.water_change_interval_days && (
                  <>
                    <span className="text-emerald-300">|</span>
                    <span>Thay nước: <b>mỗi {currentStage.water_change_interval_days} ngày</b></span>
                  </>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </section>
  );
};