import { useState, useEffect, useCallback, useMemo } from 'react';

export interface OnboardingState {
  completedSteps: string[];
  dismissed: boolean;
  firstDevicePaired: boolean;
  firstDataReceived: boolean;
  firstSeasonCreated: boolean;
}

export const ONBOARDING_STORAGE_KEY = 'hydragrow_onboarding';

export const DEFAULT_ONBOARDING_STATE: OnboardingState = {
  completedSteps: [],
  dismissed: false,
  firstDevicePaired: false,
  firstDataReceived: false,
  firstSeasonCreated: false,
};

export const ONBOARDING_TOTAL_STEPS = 4;

function loadStoredState(): OnboardingState {
  if (typeof window === 'undefined' || !window.localStorage) {
    return DEFAULT_ONBOARDING_STATE;
  }
  try {
    const raw = window.localStorage.getItem(ONBOARDING_STORAGE_KEY);
    if (!raw) return DEFAULT_ONBOARDING_STATE;
    const parsed = JSON.parse(raw);
    return {
      completedSteps: Array.isArray(parsed.completedSteps) ? parsed.completedSteps : [],
      dismissed: Boolean(parsed.dismissed),
      firstDevicePaired: Boolean(parsed.firstDevicePaired),
      firstDataReceived: Boolean(parsed.firstDataReceived),
      firstSeasonCreated: Boolean(parsed.firstSeasonCreated),
    };
  } catch {
    return DEFAULT_ONBOARDING_STATE;
  }
}

function persistState(state: OnboardingState): void {
  if (typeof window === 'undefined' || !window.localStorage) return;
  try {
    window.localStorage.setItem(ONBOARDING_STORAGE_KEY, JSON.stringify(state));
  } catch {
    // Storage quota or sandboxed iframe error handling
  }
}

export function useOnboardingState() {
  const [state, setState] = useState<OnboardingState>(() => loadStoredState());

  // Listen for storage events across tabs
  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key === ONBOARDING_STORAGE_KEY) {
        setState(loadStoredState());
      }
    };
    window.addEventListener('storage', handleStorage);
    return () => window.removeEventListener('storage', handleStorage);
  }, []);

  const updateState = useCallback((updater: (prev: OnboardingState) => OnboardingState) => {
    setState((prev) => {
      const next = updater(prev);
      persistState(next);
      return next;
    });
  }, []);

  const completeStep = useCallback((stepId: string) => {
    updateState((prev) => {
      if (prev.completedSteps.includes(stepId)) {
        return prev;
      }
      const updatedSteps = [...prev.completedSteps, stepId];
      const isDeviceStep = stepId === 'pair_device' || stepId === 'step_2';
      const isDataStep = stepId === 'first_data' || stepId === 'step_3';
      const isSeasonStep = stepId === 'first_season' || stepId === 'step_4';

      return {
        ...prev,
        completedSteps: updatedSteps,
        firstDevicePaired: prev.firstDevicePaired || isDeviceStep,
        firstDataReceived: prev.firstDataReceived || isDataStep,
        firstSeasonCreated: prev.firstSeasonCreated || isSeasonStep,
      };
    });
  }, [updateState]);

  const dismiss = useCallback(() => {
    updateState((prev) => ({ ...prev, dismissed: true }));
  }, [updateState]);

  const reset = useCallback(() => {
    updateState(() => DEFAULT_ONBOARDING_STATE);
  }, [updateState]);

  const isStepComplete = useCallback(
    (stepId: string) => {
      return state.completedSteps.includes(stepId);
    },
    [state.completedSteps]
  );

  const shouldShowOnboarding = useMemo(() => {
    return !state.dismissed && state.completedSteps.length < ONBOARDING_TOTAL_STEPS;
  }, [state.dismissed, state.completedSteps.length]);

  return {
    state,
    completedSteps: state.completedSteps,
    dismissed: state.dismissed,
    firstDevicePaired: state.firstDevicePaired,
    firstDataReceived: state.firstDataReceived,
    firstSeasonCreated: state.firstSeasonCreated,
    completeStep,
    dismiss,
    reset,
    isStepComplete,
    shouldShowOnboarding,
  };
}
