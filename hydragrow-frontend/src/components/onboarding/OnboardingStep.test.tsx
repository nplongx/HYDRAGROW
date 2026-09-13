import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { OnboardingStep } from './OnboardingStep';

describe('OnboardingStep', () => {
  it('renders step with title and description', () => {
    render(
      <OnboardingStep
        stepNumber={1}
        title="Chào mừng bạn"
        description="Mô tả hướng dẫn bước 1"
      />
    );

    expect(screen.getByText('Chào mừng bạn')).toBeInTheDocument();
    expect(screen.getByText('Mô tả hướng dẫn bước 1')).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument();
  });

  it('renders action buttons and handles clicks when not completed', () => {
    const handlePrimary = vi.fn();
    const handleSecondary = vi.fn();

    render(
      <OnboardingStep
        stepNumber={2}
        title="Bước 2"
        description="Mô tả bước 2"
        primaryAction={{ label: 'Thực hiện', onClick: handlePrimary }}
        secondaryAction={{ label: 'Bỏ qua', onClick: handleSecondary }}
        isActive={true}
      />
    );

    const primaryBtn = screen.getByRole('button', { name: 'Thực hiện' });
    const secondaryBtn = screen.getByRole('button', { name: 'Bỏ qua' });

    fireEvent.click(primaryBtn);
    expect(handlePrimary).toHaveBeenCalledTimes(1);

    fireEvent.click(secondaryBtn);
    expect(handleSecondary).toHaveBeenCalledTimes(1);
  });

  it('renders completed state with checkmark and hides actions', () => {
    const handlePrimary = vi.fn();

    render(
      <OnboardingStep
        stepNumber={3}
        title="Bước 3 đã xong"
        description="Mô tả bước 3"
        isComplete={true}
        primaryAction={{ label: 'Thực hiện', onClick: handlePrimary }}
      />
    );

    expect(screen.getByTestId('step-completed-badge')).toBeInTheDocument();
    expect(screen.getByText('Đã xong')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Thực hiện' })).not.toBeInTheDocument();
  });
});
