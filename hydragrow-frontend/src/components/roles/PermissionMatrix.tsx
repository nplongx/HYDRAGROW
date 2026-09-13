import { useState, type KeyboardEvent } from 'react';
import { Check, X } from 'lucide-react';
import { RoleBadge, type UserRole, ROLE_DISPLAY_NAMES } from './RoleBadge';

export interface Capability {
  id: string;
  title: string;
  description: string;
  admin?: boolean;
  operator?: boolean;
  viewer?: boolean;
  [key: string]: any;
}

export const DEFAULT_CAPABILITIES: Capability[] = [
  {
    id: 'telemetry',
    title: 'Giám sát & Số liệu (Telemetry)',
    description: 'Xem thông số cảm biến thời gian thực, biểu đồ lịch sử và nhật ký sự kiện',
    admin: true,
    operator: true,
    viewer: true,
  },
  {
    id: 'control',
    title: 'Điều khiển & Vận hành (Pumps / E-Stop)',
    description: 'Bật/tắt bơm dinh dưỡng, phun sương, xả tràn và kích hoạt dừng khẩn cấp E-stop',
    admin: true,
    operator: true,
    viewer: false,
  },
  {
    id: 'recipes',
    title: 'Cấu hình nông học & Kịch bản (Recipes / Config)',
    description: 'Tạo và chỉnh sửa công thức mùa vụ, ngưỡng pH/EC, lịch trình chiếu sáng',
    admin: true,
    operator: true,
    viewer: false,
  },
  {
    id: 'device',
    title: 'OTA & Mạng thiết bị (OTA & Device Network)',
    description: 'Cập nhật firmware OTA, thiết lập mạng WiFi trạm',
    admin: true,
    operator: true,
    viewer: false,
  },
  {
    id: 'permissions',
    title: 'Phân quyền & Mời thành viên (Permissions & Member Invitation)',
    description: 'Mời và đổi vai trò thành viên',
    admin: true,
    operator: false,
    viewer: false,
  },
];

export const DEFAULT_ROLES: UserRole[] = ['admin', 'operator', 'viewer'];

export interface PermissionMatrixProps {
  /** List of system capabilities to display */
  capabilities?: Capability[];
  /** Array of active roles to present as columns */
  roles?: UserRole[];
  /** Compact presentation with tighter padding */
  compact?: boolean;
  /** Optional custom class name */
  className?: string;
  /** Test identifier */
  'data-testid'?: string;
}

/**
 * PermissionMatrix displays a responsive, accessible matrix of capabilities vs roles.
 * Supports:
 * - ARIA role="grid" with full keyboard navigation (arrow keys, Home, End)
 * - Accessible aria-labels on every capability cell
 * - Responsive collapsing to stacked cards on viewports < 640px
 */
export function PermissionMatrix({
  capabilities = DEFAULT_CAPABILITIES,
  roles = DEFAULT_ROLES,
  compact = false,
  className = '',
  'data-testid': testId = 'permission-matrix',
}: PermissionMatrixProps) {
  const [focusedCoords, setFocusedCoords] = useState<{ row: number; col: number }>({
    row: 0,
    col: 0,
  });

  const handleKeyDown = (
    e: KeyboardEvent,
    rowIndex: number,
    colIndex: number
  ) => {
    let nextRow = rowIndex;
    let nextCol = colIndex;
    let handled = false;

    switch (e.key) {
      case 'ArrowUp':
        nextRow = Math.max(0, rowIndex - 1);
        handled = true;
        break;
      case 'ArrowDown':
        nextRow = Math.min(capabilities.length - 1, rowIndex + 1);
        handled = true;
        break;
      case 'ArrowLeft':
        nextCol = Math.max(0, colIndex - 1);
        handled = true;
        break;
      case 'ArrowRight':
        nextCol = Math.min(roles.length - 1, colIndex + 1);
        handled = true;
        break;
      case 'Home':
        nextCol = 0;
        handled = true;
        break;
      case 'End':
        nextCol = roles.length - 1;
        handled = true;
        break;
      default:
        break;
    }

    if (handled) {
      e.preventDefault();
      setFocusedCoords({ row: nextRow, col: nextCol });
      const targetId = `perm-cell-${nextRow}-${nextCol}`;
      const el = document.getElementById(targetId);
      el?.focus();
    }
  };

  if (capabilities.length === 0) {
    return (
      <div
        data-testid={testId}
        className={`py-8 text-center text-sm text-text-muted ${className}`.trim()}
      >
        Chưa có năng lực nào được khai báo
      </div>
    );
  }

  return (
    <div className={`overflow-x-auto ${className}`.trim()}>
      <table
        role="grid"
        aria-label="Ma trận phân quyền năng lực theo vai trò"
        data-testid={testId}
        className="w-full text-left border-collapse block sm:table"
      >
        <thead className="hidden sm:table-header-group">
          <tr
            role="row"
            className="border-b border-line text-xs font-semibold text-text-muted uppercase tracking-wider"
          >
            <th role="columnheader" scope="col" className="py-3 px-4 w-1/2">
              Năng lực hệ thống
            </th>
            {roles.map((role) => (
              <th
                key={role}
                role="columnheader"
                scope="col"
                className="py-3 px-4 text-center"
              >
                <div className="flex items-center justify-center">
                  <RoleBadge role={role} size="sm" />
                </div>
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="block sm:table-row-group divide-y sm:divide-y divide-line text-sm">
          {capabilities.map((cap, rowIndex) => (
            <tr
              key={cap.id}
              role="row"
              className="block sm:table-row p-3 sm:p-0 mb-3 sm:mb-0 border sm:border-0 border-line rounded-xl sm:rounded-none bg-white sm:bg-transparent hover:bg-soft/30 transition-colors"
            >
              {/* Capability title and description */}
              <td
                role="rowheader"
                scope="row"
                className={`block sm:table-cell py-2 sm:py-3 px-1 sm:px-4 mb-2 sm:mb-0 ${
                  compact ? 'sm:py-2' : ''
                }`}
              >
                <div className="font-semibold text-primary-deep">{cap.title}</div>
                <div className="text-xs text-text-muted mt-0.5">{cap.description}</div>
              </td>

              {/* Status cells for each role */}
              {roles.map((role, colIndex) => {
                const hasAccess = Boolean(cap[role]);
                const roleName = ROLE_DISPLAY_NAMES[role] || role;
                const isFocused =
                  focusedCoords.row === rowIndex && focusedCoords.col === colIndex;
                const cellId = `perm-cell-${rowIndex}-${colIndex}`;
                const ariaLabel = `${roleName}: ${
                  hasAccess ? 'Có quyền' : 'Không có quyền'
                } ${cap.title}`;

                return (
                  <td
                    key={role}
                    id={cellId}
                    role="gridcell"
                    tabIndex={isFocused ? 0 : -1}
                    aria-label={ariaLabel}
                    onKeyDown={(e) => handleKeyDown(e, rowIndex, colIndex)}
                    onClick={() => setFocusedCoords({ row: rowIndex, col: colIndex })}
                    className={`block sm:table-cell py-1.5 sm:py-3 px-2 sm:px-4 sm:text-center transition-colors focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-1 rounded-md ${
                      compact ? 'sm:py-2 sm:px-2' : ''
                    }`}
                  >
                    <div className="flex items-center justify-between sm:justify-center">
                      <span className="text-xs text-text-muted font-medium sm:hidden">
                        {roleName}:
                      </span>
                      {hasAccess ? (
                        <span
                          className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-pill text-status"
                          aria-hidden="true"
                        >
                          <Check size={14} />
                        </span>
                      ) : (
                        <span
                          className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-surface-muted text-text-muted"
                          aria-hidden="true"
                        >
                          <X size={14} />
                        </span>
                      )}
                    </div>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export default PermissionMatrix;
