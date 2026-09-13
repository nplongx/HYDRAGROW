// src/components/safety/EmergencyStopButton.guard.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { EmergencyStopButton } from './EmergencyStopButton';

describe('EmergencyStopButton — Fitts\'s Law / Von Restorff regression guard', () => {
  it('touch target is at least 48px (w-14/h-14 or larger)', () => {
    render(<EmergencyStopButton deviceId={null} variant="floating" />);
    const button = screen.getByRole('button', { name: /dừng khẩn cấp|emergency/i });
    const sizeClasses = ['w-14', 'h-14', 'w-16', 'h-16'];
    expect(sizeClasses.some((c) => button.className.includes(c))).toBe(true);
  });

  it('no other bg-red-*/bg-danger/bg-error element exists on Dashboard.tsx or Operations.tsx source (Von Restorff uniqueness)', () => {
    const dashboardSrc = fs.readFileSync(path.resolve(__dirname, '../../pages/Dashboard.tsx'), 'utf-8');
    const operationsSrc = fs.readFileSync(path.resolve(__dirname, '../../pages/Operations.tsx'), 'utf-8');
    const dangerPattern = /bg-red-\d{2,3}|bg-danger|bg-error/;
    expect(dangerPattern.test(dashboardSrc)).toBe(false);
    expect(dangerPattern.test(operationsSrc)).toBe(false);
  });
});
