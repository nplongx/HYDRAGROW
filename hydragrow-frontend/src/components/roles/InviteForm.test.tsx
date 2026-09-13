import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { InviteForm } from './InviteForm';

describe('InviteForm', () => {
  const defaultProps = {
    onSubmit: vi.fn().mockResolvedValue(undefined),
    onCancel: vi.fn(),
    isSubmitting: false,
  };

  it('renders 3 groups: Identity, Role Assignment, Confirmation', () => {
    render(<InviteForm {...defaultProps} />);

    // Check group legends / testids
    expect(screen.getByTestId('group-identity')).toBeInTheDocument();
    expect(screen.getByTestId('group-role')).toBeInTheDocument();
    expect(screen.getByTestId('group-confirmation')).toBeInTheDocument();

    expect(screen.getByText(/Identity/i)).toBeInTheDocument();
    expect(screen.getByText(/Role Assignment/i)).toBeInTheDocument();
    expect(screen.getByText(/Confirmation/i)).toBeInTheDocument();
  });

  it('validates Firebase UID on blur with correct error message and aria-invalid/aria-describedby', async () => {
    render(<InviteForm {...defaultProps} />);

    const uidInput = screen.getByLabelText(/Firebase UID/i);
    expect(uidInput).not.toHaveAttribute('aria-invalid', 'true');

    // Trigger blur on empty input
    fireEvent.focus(uidInput);
    fireEvent.blur(uidInput);

    expect(screen.getByText('UID phải chứa ký tự chữ-số, 1-128 ký tự')).toBeInTheDocument();
    expect(uidInput).toHaveAttribute('aria-invalid', 'true');
    expect(uidInput).toHaveAttribute('aria-describedby', 'uid-error');

    // Trigger with special characters (e.g. invalid spaces / dashes)
    fireEvent.change(uidInput, { target: { value: 'invalid-uid@!' } });
    fireEvent.blur(uidInput);

    expect(screen.getByText('UID phải chứa ký tự chữ-số, 1-128 ký tự')).toBeInTheDocument();
    expect(uidInput).toHaveAttribute('aria-invalid', 'true');

    // Valid alphanumeric UID
    fireEvent.change(uidInput, { target: { value: 'FirebaseUidValid123' } });
    fireEvent.blur(uidInput);

    expect(screen.queryByText('UID phải chứa ký tự chữ-số, 1-128 ký tự')).not.toBeInTheDocument();
    expect(uidInput).not.toHaveAttribute('aria-invalid', 'true');
  });

  it('validates Email on blur with correct error message and aria-invalid/aria-describedby', async () => {
    render(<InviteForm {...defaultProps} />);

    const emailInput = screen.getByLabelText(/Email tài khoản/i);
    expect(emailInput).not.toHaveAttribute('aria-invalid', 'true');

    // Blur on empty
    fireEvent.focus(emailInput);
    fireEvent.blur(emailInput);

    expect(screen.getByText('Email không hợp lệ')).toBeInTheDocument();
    expect(emailInput).toHaveAttribute('aria-invalid', 'true');
    expect(emailInput).toHaveAttribute('aria-describedby', 'email-error');

    // Blur on invalid format
    fireEvent.change(emailInput, { target: { value: 'not-an-email' } });
    fireEvent.blur(emailInput);

    expect(screen.getByText('Email không hợp lệ')).toBeInTheDocument();
    expect(emailInput).toHaveAttribute('aria-invalid', 'true');

    // Valid email
    fireEvent.change(emailInput, { target: { value: 'user@farm.vn' } });
    fireEvent.blur(emailInput);

    expect(screen.queryByText('Email không hợp lệ')).not.toBeInTheDocument();
    expect(emailInput).not.toHaveAttribute('aria-invalid', 'true');
  });

  it('role selection uses radio buttons with scope descriptions', () => {
    render(<InviteForm {...defaultProps} />);

    // Radio buttons
    const radioOperator = screen.getByRole('radio', { name: /Vận hành viên/i });
    const radioViewer = screen.getByRole('radio', { name: /Người xem/i });
    const radioAdmin = screen.getByRole('radio', { name: /Quản trị viên/i });

    expect(radioOperator).toBeInTheDocument();
    expect(radioViewer).toBeInTheDocument();
    expect(radioAdmin).toBeInTheDocument();

    // Default selection
    expect(radioOperator).toBeChecked();

    // Scope descriptions
    expect(screen.getByText('Điều khiển bơm, kịch bản, OTA')).toBeInTheDocument();
    expect(screen.getByText('Chỉ xem số liệu')).toBeInTheDocument();
    expect(screen.getByText('Toàn quyền truy cập')).toBeInTheDocument();

    // Switching radio updates checked state
    fireEvent.click(radioAdmin);
    expect(radioAdmin).toBeChecked();
    expect(radioOperator).not.toBeChecked();
  });

  it('submit calls onSubmit with correct data structure', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<InviteForm {...defaultProps} onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText(/Firebase UID/i), {
      target: { value: 'alphaNumericUid123' },
    });
    fireEvent.change(screen.getByLabelText(/Email tài khoản/i), {
      target: { value: 'farmer@farm.vn' },
    });
    fireEvent.change(screen.getByLabelText(/Tên hiển thị/i), {
      target: { value: 'Nguyễn Văn A' },
    });

    // Select admin role
    fireEvent.click(screen.getByRole('radio', { name: /Quản trị viên/i }));

    fireEvent.click(screen.getByRole('button', { name: /Xác nhận cấp quyền/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledTimes(1);
      expect(onSubmit).toHaveBeenCalledWith({
        firebase_uid: 'alphaNumericUid123',
        email: 'farmer@farm.vn',
        display_name: 'Nguyễn Văn A',
        role: 'admin',
      });
    });
  });

  it('submitting with invalid fields shows errors and does not call onSubmit', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<InviteForm {...defaultProps} onSubmit={onSubmit} />);

    // Attempt submit with empty fields
    fireEvent.click(screen.getByRole('button', { name: /Xác nhận cấp quyền/i }));

    await waitFor(() => {
      expect(screen.getByText('UID phải chứa ký tự chữ-số, 1-128 ký tự')).toBeInTheDocument();
      expect(screen.getByText('Email không hợp lệ')).toBeInTheDocument();
      expect(onSubmit).not.toHaveBeenCalled();
    });
  });

  it('cancel calls onCancel', () => {
    const onCancel = vi.fn();
    render(<InviteForm {...defaultProps} onCancel={onCancel} />);

    fireEvent.click(screen.getByRole('button', { name: /Huỷ/i }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('shows error banner on submit failure and keeps form filled for recovery', async () => {
    const onSubmit = vi.fn().mockRejectedValue(new Error('Tài khoản đã tồn tại'));
    render(<InviteForm {...defaultProps} onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText(/Firebase UID/i), {
      target: { value: 'validUid123' },
    });
    fireEvent.change(screen.getByLabelText(/Email tài khoản/i), {
      target: { value: 'existing@farm.vn' },
    });

    fireEvent.click(screen.getByRole('button', { name: /Xác nhận cấp quyền/i }));

    await waitFor(() => {
      expect(screen.getByText('Tài khoản đã tồn tại')).toBeInTheDocument();
    });

    // Form inputs remain filled for user correction
    expect(screen.getByLabelText(/Firebase UID/i)).toHaveValue('validUid123');
    expect(screen.getByLabelText(/Email tài khoản/i)).toHaveValue('existing@farm.vn');
  });

  it('summary panel updates email, role, and scope list dynamically', () => {
    render(<InviteForm {...defaultProps} />);

    expect(screen.getByTestId('summary-email')).toHaveTextContent('(Chưa nhập)');
    expect(screen.getByTestId('summary-role')).toHaveTextContent('Vận hành viên');

    // Type email
    fireEvent.change(screen.getByLabelText(/Email tài khoản/i), {
      target: { value: 'preview@farm.vn' },
    });
    expect(screen.getByTestId('summary-email')).toHaveTextContent('preview@farm.vn');

    // Switch to viewer
    fireEvent.click(screen.getByRole('radio', { name: /Người xem/i }));
    expect(screen.getByTestId('summary-role')).toHaveTextContent('Người xem');
    expect(screen.getByTestId('summary-scopes')).toHaveTextContent('read:telemetry');
  });
});
