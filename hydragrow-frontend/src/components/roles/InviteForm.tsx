import React, { useState } from 'react';
import { Banner } from '../ui/Banner';
import { RoleBadge } from './RoleBadge';
import {
  ROLE_DEFAULT_SCOPES,
  ROLE_DISPLAY_NAMES,
  type UserRole,
} from '../../pages/Roles';

export interface InviteFormData {
  firebase_uid: string;
  email: string;
  display_name: string | null;
  role: UserRole;
}

export interface InviteFormProps {
  onSubmit: (data: InviteFormData) => Promise<void>;
  onCancel: () => void;
  isSubmitting?: boolean;
  errorMessage?: string | null;
  initialRole?: UserRole;
}

export const ROLE_SCOPE_DESCRIPTIONS: Record<UserRole, string> = {
  admin: 'Toàn quyền truy cập',
  operator: 'Điều khiển bơm, kịch bản, OTA',
  viewer: 'Chỉ xem số liệu',
};

const AVAILABLE_ROLES: UserRole[] = ['operator', 'viewer', 'admin'];

export function validateFirebaseUid(uid: string): string | null {
  const trimmed = uid.trim();
  if (!trimmed || !/^[a-zA-Z0-9]{1,128}$/.test(trimmed)) {
    return 'UID phải chứa ký tự chữ-số, 1-128 ký tự';
  }
  return null;
}

export function validateEmail(email: string): string | null {
  const trimmed = email.trim();
  if (!trimmed || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(trimmed)) {
    return 'Email không hợp lệ';
  }
  return null;
}

export const InviteForm: React.FC<InviteFormProps> = ({
  onSubmit,
  onCancel,
  isSubmitting = false,
  errorMessage = null,
  initialRole = 'operator',
}) => {
  const [firebaseUid, setFirebaseUid] = useState('');
  const [email, setEmail] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [role, setRole] = useState<UserRole>(initialRole);

  const [uidFieldError, setUidFieldError] = useState<string | null>(null);
  const [emailFieldError, setEmailFieldError] = useState<string | null>(null);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const handleBlurUid = () => {
    setUidFieldError(validateFirebaseUid(firebaseUid));
  };

  const handleBlurEmail = () => {
    setEmailFieldError(validateEmail(email));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const uidErr = validateFirebaseUid(firebaseUid);
    const emailErr = validateEmail(email);

    setUidFieldError(uidErr);
    setEmailFieldError(emailErr);

    if (uidErr || emailErr) {
      return;
    }

    try {
      setSubmitError(null);
      await onSubmit({
        firebase_uid: firebaseUid.trim(),
        email: email.trim(),
        display_name: displayName.trim() || null,
        role,
      });
    } catch (err: any) {
      setSubmitError(err?.message || 'Không thể thêm thành viên');
    }
  };

  const activeError = submitError || errorMessage;

  return (
    <form onSubmit={handleSubmit} className="space-y-6" noValidate>
      {activeError && (
        <Banner tone="danger" title="Không thể thêm thành viên">
          {activeError}
        </Banner>
      )}

      {/* Group 1: Identity */}
      <fieldset data-testid="group-identity" className="space-y-4">
        <legend className="text-xs font-bold uppercase tracking-wider text-text-muted">
          1. Thông tin định danh (Identity)
        </legend>

        <div>
          <label
            htmlFor="invite-firebase-uid"
            className="block text-xs font-semibold text-primary-deep mb-0.5"
          >
            Firebase UID *
          </label>
          <p className="text-[11px] text-text-muted mb-1.5">
            Lấy từ Firebase Console &gt; Authentication
          </p>
          <input
            id="invite-firebase-uid"
            type="text"
            required
            value={firebaseUid}
            onChange={(e) => {
              setFirebaseUid(e.target.value);
              if (uidFieldError) setUidFieldError(null);
            }}
            onBlur={handleBlurUid}
            placeholder="Lấy từ Firebase Console > Authentication"
            aria-invalid={Boolean(uidFieldError)}
            aria-describedby={uidFieldError ? 'uid-error' : undefined}
            className={`w-full px-3 py-2 text-sm rounded-xl border bg-white focus:outline-none font-mono text-xs ${
              uidFieldError
                ? 'border-error focus:border-error'
                : 'border-line focus:border-primary'
            }`}
          />
          {uidFieldError && (
            <p id="uid-error" className="mt-1 text-[11px] text-error font-medium">
              {uidFieldError}
            </p>
          )}
        </div>

        <div>
          <label
            htmlFor="invite-email"
            className="block text-xs font-semibold text-primary-deep mb-1"
          >
            Email tài khoản *
          </label>
          <input
            id="invite-email"
            type="email"
            required
            value={email}
            onChange={(e) => {
              setEmail(e.target.value);
              if (emailFieldError) setEmailFieldError(null);
            }}
            onBlur={handleBlurEmail}
            placeholder="operator@farm.vn"
            aria-invalid={Boolean(emailFieldError)}
            aria-describedby={emailFieldError ? 'email-error' : undefined}
            className={`w-full px-3 py-2 text-sm rounded-xl border bg-white focus:outline-none ${
              emailFieldError
                ? 'border-error focus:border-error'
                : 'border-line focus:border-primary'
            }`}
          />
          {emailFieldError && (
            <p id="email-error" className="mt-1 text-[11px] text-error font-medium">
              {emailFieldError}
            </p>
          )}
        </div>

        <div>
          <label
            htmlFor="invite-display-name"
            className="block text-xs font-semibold text-primary-deep mb-1"
          >
            Tên hiển thị
          </label>
          <input
            id="invite-display-name"
            type="text"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder="Kỹ sư nông học A"
            className="w-full px-3 py-2 text-sm rounded-xl border border-line bg-white focus:outline-none focus:border-primary"
          />
        </div>
      </fieldset>

      {/* Group 2: Role Assignment */}
      <fieldset data-testid="group-role" className="space-y-3">
        <legend className="text-xs font-bold uppercase tracking-wider text-text-muted">
          2. Phân bổ vai trò (Role Assignment)
        </legend>

        <div className="space-y-2">
          {AVAILABLE_ROLES.map((r) => {
            const isSelected = role === r;
            return (
              <label
                key={r}
                htmlFor={`role-option-${r}`}
                className={`flex items-start gap-3 p-3 rounded-xl border cursor-pointer transition-colors ${
                  isSelected
                    ? 'border-primary bg-pill/30'
                    : 'border-line bg-white hover:bg-soft/50'
                }`}
              >
                <input
                  type="radio"
                  id={`role-option-${r}`}
                  name="role-assignment"
                  value={r}
                  checked={isSelected}
                  onChange={() => setRole(r)}
                  className="mt-1 h-4 w-4 text-primary border-line focus:ring-primary cursor-pointer"
                />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-semibold text-primary-deep">
                      {ROLE_DISPLAY_NAMES[r]}
                    </span>
                    {r === 'operator' && (
                      <span className="text-[10px] font-medium text-primary bg-primary/10 px-1.5 py-0.5 rounded">
                        Khuyên dùng
                      </span>
                    )}
                    <RoleBadge role={r} size="sm" />
                  </div>
                  <p className="text-xs text-text-muted mt-0.5">
                    {ROLE_SCOPE_DESCRIPTIONS[r]}
                  </p>
                </div>
              </label>
            );
          })}
        </div>
      </fieldset>

      {/* Group 3: Confirmation */}
      <fieldset data-testid="group-confirmation" className="space-y-4">
        <legend className="text-xs font-bold uppercase tracking-wider text-text-muted">
          3. Xác nhận &amp; Cấp quyền (Confirmation)
        </legend>

        <div className="p-4 rounded-xl bg-soft/60 border border-line space-y-2">
          <h4 className="text-xs font-bold text-primary-deep uppercase tracking-wider">
            Tóm tắt thông tin phân quyền
          </h4>
          <div className="text-xs space-y-1.5 text-text-muted">
            <div className="flex items-center justify-between">
              <span>Email:</span>
              <span className="font-semibold text-primary-deep font-mono" data-testid="summary-email">
                {email.trim() || '(Chưa nhập)'}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span>Vai trò:</span>
              <span className="font-semibold text-primary-deep" data-testid="summary-role">
                {ROLE_DISPLAY_NAMES[role]}
              </span>
            </div>
            <div>
              <span className="block mb-1">Tập quyền (scopes):</span>
              <div className="flex flex-wrap gap-1" data-testid="summary-scopes">
                {ROLE_DEFAULT_SCOPES[role].map((scope) => (
                  <code
                    key={scope}
                    className="px-1.5 py-0.5 bg-white border border-line rounded text-[11px] font-mono text-primary-deep"
                  >
                    {scope}
                  </code>
                ))}
              </div>
            </div>
          </div>
        </div>

        <div className="pt-2 flex items-center justify-end gap-3">
          <button
            type="button"
            onClick={onCancel}
            disabled={isSubmitting}
            className="ui-btn-md border border-line text-primary-deep bg-white hover:bg-soft"
          >
            Huỷ
          </button>
          <button
            type="submit"
            disabled={isSubmitting}
            className="ui-btn-primary"
          >
            {isSubmitting ? 'Đang thêm...' : 'Xác nhận cấp quyền'}
          </button>
        </div>
      </fieldset>
    </form>
  );
};
