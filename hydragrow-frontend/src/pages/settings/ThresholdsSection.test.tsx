import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { ThresholdsSection } from './ThresholdsSection';

describe('ThresholdsSection', () => {
  it('render đủ 5 accordion con: Ngưỡng mục tiêu, Quản lý nước, Máy châm phân, An toàn, Cảm biến', () => {
    render(
      <ThresholdsSection
        openSection={null}
        onToggleSection={() => {}}
        config={{}}
        setConfig={() => {}}
        isAdvancedMode={true}
        dosingValidationErrors={{}}
        wizardStep={0}
        calibrationPoints={[7, 4]}
        activePoint={7}
        isCalibrationBlocked={false}
        isCapturingPoint={false}
        countdown={0}
        capturedPoints={{}}
        calibrationSummary={{ ph_v7: null, ph_v4: null, reliability: 0 }}
        handleCapturePoint={() => {}}
        goToNextPoint={() => {}}
        handleFinishAndSaveCalibration={() => {}}
      />
    );
    expect(screen.getByText('Ngưỡng mục tiêu')).toBeInTheDocument();
    expect(screen.getByText('Quản lý nước')).toBeInTheDocument();
    expect(screen.getByText('Máy châm phân')).toBeInTheDocument();
    expect(screen.getByText('An toàn')).toBeInTheDocument();
    expect(screen.getByText('Cảm biến & Hiệu chuẩn')).toBeInTheDocument();
  });
});
