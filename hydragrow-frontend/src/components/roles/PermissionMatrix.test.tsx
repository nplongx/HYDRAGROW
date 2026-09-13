import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import {
  PermissionMatrix,
  type Capability,
} from './PermissionMatrix';
import type { UserRole } from './RoleBadge';

describe('PermissionMatrix', () => {
  it('renders table with role="grid" and default capabilities and roles', () => {
    render(<PermissionMatrix />);

    const grid = screen.getByRole('grid', { name: /Ma trận phân quyền năng lực theo vai trò/i });
    expect(grid).toBeInTheDocument();

    // Check all default capabilities are rendered
    expect(screen.getByText('Giám sát & Số liệu (Telemetry)')).toBeInTheDocument();
    expect(screen.getByText('Điều khiển & Vận hành (Pumps / E-Stop)')).toBeInTheDocument();
    expect(screen.getByText('Cấu hình nông học & Kịch bản (Recipes / Config)')).toBeInTheDocument();
    expect(screen.getByText('OTA & Mạng thiết bị (OTA & Device Network)')).toBeInTheDocument();
    expect(screen.getByText('Phân quyền & Mời thành viên (Permissions & Member Invitation)')).toBeInTheDocument();
  });

  it('renders custom capabilities and roles passed as props', () => {
    const customCaps: Capability[] = [
      {
        id: 'irrigation',
        title: 'Tưới tiêu thông minh',
        description: 'Tự động kích hoạt van tưới theo độ ẩm',
        admin: true,
        operator: true,
      },
    ];
    const customRoles: UserRole[] = ['admin', 'operator'];

    render(<PermissionMatrix capabilities={customCaps} roles={customRoles} />);

    expect(screen.getByText('Tưới tiêu thông minh')).toBeInTheDocument();
    expect(screen.getByText('Tự động kích hoạt van tưới theo độ ẩm')).toBeInTheDocument();

    // Only custom roles rendered in columnheaders
    const headers = screen.getAllByRole('columnheader');
    expect(headers).toHaveLength(3); // 1 for "Năng lực hệ thống" + 2 roles
    expect(headers[1]).toHaveTextContent('Quản trị viên');
    expect(headers[2]).toHaveTextContent('Vận hành viên');
  });

  it('provides accessible aria-label on capability gridcells', () => {
    render(<PermissionMatrix />);

    // Check cells for first capability: telemetry (admin=true, operator=true, viewer=true)
    const adminTelemetryCell = screen.getByLabelText(
      'Quản trị viên: Có quyền Giám sát & Số liệu (Telemetry)'
    );
    expect(adminTelemetryCell).toBeInTheDocument();
    expect(adminTelemetryCell).toHaveAttribute('role', 'gridcell');

    // Check cell with false permission: control for viewer
    const viewerControlCell = screen.getByLabelText(
      'Người xem: Không có quyền Điều khiển & Vận hành (Pumps / E-Stop)'
    );
    expect(viewerControlCell).toBeInTheDocument();
    expect(viewerControlCell).toHaveAttribute('role', 'gridcell');
  });

  it('supports keyboard navigation across grid cells with arrow keys', () => {
    render(<PermissionMatrix />);

    const firstCell = screen.getByLabelText(
      'Quản trị viên: Có quyền Giám sát & Số liệu (Telemetry)'
    );
    const secondCell = screen.getByLabelText(
      'Vận hành viên: Có quyền Giám sát & Số liệu (Telemetry)'
    );
    const cellBelow = screen.getByLabelText(
      'Quản trị viên: Có quyền Điều khiển & Vận hành (Pumps / E-Stop)'
    );

    // Initial cell has tabIndex 0
    expect(firstCell).toHaveAttribute('tabIndex', '0');
    expect(secondCell).toHaveAttribute('tabIndex', '-1');

    // Focus first cell
    firstCell.focus();
    expect(document.activeElement).toBe(firstCell);

    // Navigate right with ArrowRight
    fireEvent.keyDown(firstCell, { key: 'ArrowRight' });
    expect(secondCell).toHaveAttribute('tabIndex', '0');
    expect(firstCell).toHaveAttribute('tabIndex', '-1');
    expect(document.activeElement).toBe(secondCell);

    // Navigate down with ArrowDown
    const secondCellBelow = screen.getByLabelText(
      'Vận hành viên: Có quyền Điều khiển & Vận hành (Pumps / E-Stop)'
    );
    fireEvent.keyDown(secondCell, { key: 'ArrowDown' });
    expect(secondCellBelow).toHaveAttribute('tabIndex', '0');
    expect(document.activeElement).toBe(secondCellBelow);

    // Navigate left with ArrowLeft
    fireEvent.keyDown(secondCellBelow, { key: 'ArrowLeft' });
    expect(cellBelow).toHaveAttribute('tabIndex', '0');
    expect(document.activeElement).toBe(cellBelow);

    // Navigate up with ArrowUp
    fireEvent.keyDown(cellBelow, { key: 'ArrowUp' });
    expect(firstCell).toHaveAttribute('tabIndex', '0');
    expect(document.activeElement).toBe(firstCell);
  });

  it('renders fallback empty state when capabilities array is empty', () => {
    render(<PermissionMatrix capabilities={[]} />);
    expect(screen.getByText('Chưa có năng lực nào được khai báo')).toBeInTheDocument();
  });
});
