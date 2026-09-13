import React, { ReactNode } from 'react';
import { CheckCircle2 } from 'lucide-react';
import { Button } from '../ui/Button';

export interface OnboardingAction {
  label: string;
  onClick: () => void;
  disabled?: boolean;
}

export interface OnboardingStepProps {
  stepNumber?: number;
  title: string;
  description: string;
  icon?: ReactNode;
  primaryAction?: OnboardingAction;
  secondaryAction?: OnboardingAction;
  isComplete?: boolean;
  isActive?: boolean;
  className?: string;
}

export const OnboardingStep: React.FC<OnboardingStepProps> = ({
  stepNumber,
  title,
  description,
  icon,
  primaryAction,
  secondaryAction,
  isComplete = false,
  isActive = false,
  className = '',
}) => {
  return (
    <div
      data-testid={`onboarding-step-${stepNumber ?? 'item'}`}
      className={`relative flex items-start gap-4 rounded-xl border p-4 transition-all ${
        isComplete
          ? 'border-line bg-surface/50 opacity-90'
          : isActive
          ? 'border-primary bg-pill/30 shadow-sm'
          : 'border-line bg-surface'
      } ${className}`}
    >
      {/* Icon / Step indicator */}
      <div className="shrink-0 pt-0.5">
        {isComplete ? (
          <div
            data-testid="step-completed-badge"
            className="flex h-8 w-8 items-center justify-center rounded-full bg-pill text-status"
            aria-label="Đã hoàn thành"
          >
            <CheckCircle2 size={18} className="stroke-[2.5]" />
          </div>
        ) : (
          <div
            className={`flex h-8 w-8 items-center justify-center rounded-full text-xs font-bold transition-colors ${
              isActive
                ? 'bg-primary-deep text-white shadow-xs'
                : 'bg-page-bg text-text-muted border border-line'
            }`}
          >
            {icon ? (
              <span className="flex items-center justify-center">{icon}</span>
            ) : (
              <span>{stepNumber ?? '•'}</span>
            )}
          </div>
        )}
      </div>

      {/* Text details & Actions */}
      <div className="min-w-0 flex-1">
        <div className="flex flex-col sm:flex-row sm:items-baseline sm:justify-between gap-1">
          <h4
            className={`text-sm font-semibold tracking-tight transition-colors ${
              isComplete
                ? 'text-text-muted line-through decoration-status/40'
                : isActive
                ? 'text-primary-deep'
                : 'text-text'
            }`}
          >
            {title}
          </h4>
          {isComplete && (
            <span className="text-[11px] font-medium text-status uppercase tracking-wider">
              Đã xong
            </span>
          )}
        </div>

        <p className="mt-1 text-xs leading-relaxed text-text-muted">{description}</p>

        {/* Action button row */}
        {(primaryAction || secondaryAction) && !isComplete && (
          <div className="mt-3 flex flex-wrap items-center gap-2 pt-0.5">
            {primaryAction && (
              <Button
                variant={isActive ? 'primary' : 'secondary'}
                size="sm"
                onClick={primaryAction.onClick}
                disabled={primaryAction.disabled}
              >
                {primaryAction.label}
              </Button>
            )}
            {secondaryAction && (
              <Button
                variant="ghost"
                size="sm"
                onClick={secondaryAction.onClick}
                disabled={secondaryAction.disabled}
              >
                {secondaryAction.label}
              </Button>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
