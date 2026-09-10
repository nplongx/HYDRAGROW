import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { LoginScreen } from './LoginScreen';
import { RegisterScreen } from './RegisterScreen';
import { ForgotPasswordScreen } from './ForgotPasswordScreen';

const loginMock = vi.fn();
const registerMock = vi.fn();
const googleMock = vi.fn();
const resetMock = vi.fn();

vi.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({
    status: 'unauthenticated',
    user: null,
    error: null,
    errorCode: null,
    errorField: 0,
    login: loginMock,
    register: registerMock,
    googleLogin: googleMock,
    resetPassword: resetMock,
    logout: vi.fn(),
  }),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe('LoginScreen', () => {
  it('gọi login khi submit email/password', async () => {
    loginMock.mockResolvedValue(undefined);
    render(<LoginScreen onShowRegister={vi.fn()} onShowForgot={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'a@b.c' } });
    fireEvent.change(screen.getByLabelText('Mật khẩu'), { target: { value: 'secret' } });
    fireEvent.click(screen.getByText('Đăng nhập'));
    await waitFor(() => expect(loginMock).toHaveBeenCalledWith('a@b.c', 'secret'));
  });

  it('có nút đăng nhập Google và liên kết đăng ký / quên mật khẩu', () => {
    render(<LoginScreen onShowRegister={vi.fn()} onShowForgot={vi.fn()} />);
    expect(screen.getByText('Đăng nhập bằng Google')).toBeInTheDocument();
    expect(screen.getByText('Đăng ký ngay')).toBeInTheDocument();
    expect(screen.getByText('Quên mật khẩu?')).toBeInTheDocument();
  });

  it('gọi googleLogin khi bấm nút Google', async () => {
    googleMock.mockResolvedValue(undefined);
    render(<LoginScreen onShowRegister={vi.fn()} onShowForgot={vi.fn()} />);
    fireEvent.click(screen.getByText('Đăng nhập bằng Google'));
    await waitFor(() => expect(googleMock).toHaveBeenCalledOnce());
  });
});

describe('RegisterScreen', () => {
  it('hiển thị lỗi khi mật khẩu xác nhận không khớp', async () => {
    render(<RegisterScreen onShowLogin={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'a@b.c' } });
    fireEvent.change(screen.getByLabelText('Mật khẩu'), { target: { value: '123456' } });
    fireEvent.change(screen.getByLabelText('Xác nhận mật khẩu'), { target: { value: '654321' } });
    fireEvent.click(screen.getByText('Đăng ký'));
    expect(await screen.findByText('Mật khẩu xác nhận không khớp.')).toBeInTheDocument();
    expect(registerMock).not.toHaveBeenCalled();
  });

  it('gọi register khi nhập hợp lệ', async () => {
    registerMock.mockResolvedValue(undefined);
    render(<RegisterScreen onShowLogin={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'a@b.c' } });
    fireEvent.change(screen.getByLabelText('Mật khẩu'), { target: { value: '123456' } });
    fireEvent.change(screen.getByLabelText('Xác nhận mật khẩu'), { target: { value: '123456' } });
    fireEvent.click(screen.getByText('Đăng ký'));
    await waitFor(() => expect(registerMock).toHaveBeenCalledWith('a@b.c', '123456'));
  });
});

describe('ForgotPasswordScreen', () => {
  it('gửi email đặt lại mật khẩu và hiển thị thông báo thành công', async () => {
    resetMock.mockResolvedValue(undefined);
    render(<ForgotPasswordScreen onShowLogin={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'a@b.c' } });
    fireEvent.click(screen.getByText('Gửi email đặt lại mật khẩu'));
    await waitFor(() => expect(resetMock).toHaveBeenCalledWith('a@b.c'));
    expect(await screen.findByText(/Đã gửi email đặt lại mật khẩu/)).toBeInTheDocument();
  });
});