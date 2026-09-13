export type UserRole = 'admin' | 'operator' | 'viewer';

export const ROLE_DISPLAY_NAMES: Record<UserRole, string> = {
  admin: 'Quản trị viên',
  operator: 'Vận hành viên',
  viewer: 'Người xem',
};

export interface RoleBadgeProps {
  /** The system role to display */
  role: UserRole;
  /** Sizing variant */
  size?: 'sm' | 'md';
  /** Optional custom class name */
  className?: string;
  /** Optional test identifier */
  'data-testid'?: string;
}

/**
 * RoleBadge standardizes role presentation pills across HydraGrow.
 * Uses semantic color tokens:
 * - admin: bg-amber-100 text-amber-800
 * - operator: bg-pill text-status
 * - viewer: bg-surface-muted text-text-muted
 */
export function RoleBadge({
  role,
  size = 'md',
  className = '',
  'data-testid': testId,
}: RoleBadgeProps) {
  const sizeClasses = size === 'sm' ? 'px-2 py-0.5 text-[11px]' : 'px-2.5 py-1 text-xs';

  const colorClasses =
    role === 'admin'
      ? 'bg-amber-100 text-amber-800'
      : role === 'operator'
      ? 'bg-pill text-status'
      : 'bg-surface-muted text-text-muted';

  const displayName = ROLE_DISPLAY_NAMES[role] || role;

  return (
    <span
      data-testid={testId}
      className={`inline-flex items-center rounded-full font-semibold ${sizeClasses} ${colorClasses} ${className}`.trim()}
    >
      {displayName}
    </span>
  );
}

export default RoleBadge;
