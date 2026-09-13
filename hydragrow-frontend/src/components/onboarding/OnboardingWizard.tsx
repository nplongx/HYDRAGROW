import React, { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Sparkles,
  X,
  Smartphone,
  Radio,
  Sprout,
  Trophy,
} from 'lucide-react';
import { useOnboardingState } from '../../hooks/useOnboardingState';
import { useDeviceStore } from '../../store/useDeviceStore';
import { OnboardingStep } from './OnboardingStep';
import { Button } from '../ui/Button';

export interface OnboardingWizardProps {
  className?: string;
  onDismiss?: () => void;
}

export const OnboardingWizard: React.FC<OnboardingWizardProps> = ({
  className = '',
  onDismiss,
}) => {
  const navigate = useNavigate();
  const {
    completedSteps,
    dismissed,
    completeStep,
    dismiss,
    isStepComplete,
  } = useOnboardingState();

  const sensorData = useDeviceStore((s) => s.sensorData);
  const ownedDevices = useDeviceStore((s) => s.ownedDevices);

  // Auto-detect step completion from live store state
  useEffect(() => {
    if (ownedDevices && ownedDevices.length > 0 && !isStepComplete('pair_device')) {
      completeStep('pair_device');
    }
  }, [ownedDevices, isStepComplete, completeStep]);

  useEffect(() => {
    if (sensorData !== null && !isStepComplete('first_data')) {
      completeStep('first_data');
    }
  }, [sensorData, isStepComplete, completeStep]);

  const stepKeys = useMemo(
    () => ['welcome', 'pair_device', 'first_data', 'first_season'],
    []
  );

  const completedCount = completedSteps.filter((s) => stepKeys.includes(s)).length;
  const isAllComplete = completedCount >= stepKeys.length;

  // Active step is the first incomplete step
  const activeStepKey = useMemo(() => {
    for (const key of stepKeys) {
      if (!isStepComplete(key)) return key;
    }
    return null;
  }, [stepKeys, isStepComplete]);

  const handleDismiss = () => {
    dismiss();
    onDismiss?.();
  };

  if (dismissed) {
    return null;
  }

  // Peak-End Rule: All 4 steps completed - celebrate!
  if (isAllComplete) {
    return (
      <div
        data-testid="onboarding-celebration"
        className={`ui-card relative overflow-hidden border-2 border-status/30 bg-pill/40 p-6 transition-all ${className}`}
      >
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <div className="flex items-start gap-3.5">
            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-status text-white shadow-sm">
              <Trophy size={26} className="animate-bounce" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="text-xl font-bold text-primary-deep">
                  🎉 Hệ thống đã sẵn sàng!
                </span>
                <span className="inline-flex items-center rounded-full bg-status/10 px-2 py-0.5 text-xs font-semibold text-status">
                  100% Hoàn tất
                </span>
              </div>
              <p className="mt-1 text-sm font-medium text-text-muted">
                Chúc vụ mùa bội thu. Bạn đã hoàn thành tất cả các bước thiết lập ban đầu!
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0 self-end sm:self-auto">
            <Button
              variant="primary"
              size="md"
              onClick={handleDismiss}
              data-testid="celebration-dismiss-btn"
            >
              Hoàn tất hướng dẫn
            </Button>
          </div>
        </div>
      </div>
    );
  }

  const progressPercent = Math.round((completedCount / stepKeys.length) * 100);

  return (
    <div
      data-testid="onboarding-wizard"
      className={`ui-card relative space-y-4 border border-line bg-surface p-5 shadow-xs transition-all ${className}`}
    >
      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-pill text-primary border border-line">
            <Sparkles size={20} />
          </div>
          <div>
            <div className="flex items-center gap-2.5">
              <h3 className="text-base font-bold text-primary-deep">
                Thiết lập hệ thống HydraGrow
              </h3>
              <span className="rounded-md bg-page-bg px-2 py-0.5 text-xs font-semibold text-text-muted">
                {completedCount}/{stepKeys.length} bước
              </span>
            </div>
            <p className="text-xs text-text-muted mt-0.5">
              Hoàn thành các bước để kích hoạt giám sát và tự động hóa toàn diện
            </p>
          </div>
        </div>

        <button
          type="button"
          onClick={handleDismiss}
          className="flex h-8 w-8 items-center justify-center rounded-lg text-text-muted hover:bg-page-bg hover:text-primary-deep transition-colors"
          aria-label="Bỏ qua hướng dẫn"
          title="Bỏ qua hướng dẫn"
        >
          <X size={18} />
        </button>
      </div>

      {/* Progress bar */}
      <div className="w-full">
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-page-bg">
          <div
            className="h-full bg-primary transition-all duration-300 ease-out"
            style={{ width: `${progressPercent}%` }}
          />
        </div>
      </div>

      {/* Steps List */}
      <div className="space-y-2.5 pt-1">
        {/* Step 1: Welcome */}
        <OnboardingStep
          stepNumber={1}
          title="Chào mừng đến HydraGrow"
          description="Nền tảng kiểm soát và tối ưu hoá dinh dưỡng khí canh chính xác cao."
          icon={<Sparkles size={16} />}
          isComplete={isStepComplete('welcome')}
          isActive={activeStepKey === 'welcome'}
          primaryAction={{
            label: 'Bắt đầu',
            onClick: () => completeStep('welcome'),
          }}
        />

        {/* Step 2: Pair Device */}
        <OnboardingStep
          stepNumber={2}
          title="Kết nối thiết bị"
          description="Ghép nối trạm cảm biến đầu tiên qua quét mã QR hoặc địa chỉ IP mạng nội bộ."
          icon={<Smartphone size={16} />}
          isComplete={isStepComplete('pair_device')}
          isActive={activeStepKey === 'pair_device'}
          primaryAction={{
            label: 'Ghép nối thiết bị',
            onClick: () => {
              completeStep('pair_device');
              navigate('/pairing');
            },
          }}
        />

        {/* Step 3: First Data */}
        <OnboardingStep
          stepNumber={3}
          title="Dữ liệu thời gian thực"
          description="Khi trạm online, bạn sẽ thấy EC, pH, nhiệt độ và mực nước cập nhật ở đây."
          icon={<Radio size={16} />}
          isComplete={isStepComplete('first_data')}
          isActive={activeStepKey === 'first_data'}
          primaryAction={{
            label: sensorData ? 'Đã nhận dữ liệu' : 'Kiểm tra tín hiệu',
            onClick: () => {
              if (sensorData) {
                completeStep('first_data');
              } else {
                navigate('/');
              }
            },
            disabled: !sensorData && isStepComplete('first_data'),
          }}
        />

        {/* Step 4: First Season */}
        <OnboardingStep
          stepNumber={4}
          title="Bắt đầu mùa vụ"
          description="Tạo công thức dinh dưỡng và chu kỳ tưới cho đợt gieo trồng đầu tiên."
          icon={<Sprout size={16} />}
          isComplete={isStepComplete('first_season')}
          isActive={activeStepKey === 'first_season'}
          primaryAction={{
            label: 'Tạo mùa vụ',
            onClick: () => {
              completeStep('first_season');
              navigate('/seasons');
            },
          }}
        />
      </div>

      {/* Footer Helper */}
      <div className="flex items-center justify-between pt-1 text-xs text-text-muted">
        <span>Bạn có thể quay lại thiết lập bất kỳ lúc nào trong Cài đặt.</span>
        <button
          type="button"
          onClick={handleDismiss}
          className="font-medium text-primary hover:underline"
        >
          Bỏ qua hướng dẫn
        </button>
      </div>
    </div>
  );
};
