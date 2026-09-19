import React, { useMemo } from "react";
import { matchPath, Outlet, useLocation, useNavigate } from "react-router-dom";
import {
  LayoutDashboard,
  SlidersHorizontal,
  Settings,
  AlignLeft,
  Leaf,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useDeviceSync } from "../../hooks/useDeviceSync";
import { useDeviceTelemetry } from "../../hooks/useDeviceTelemetry";
import { useSystemEvents } from "../../hooks/useSystemEvents";
import { useStationContext } from "../../contexts/StationContext";
import { SystemEvent } from "../../types/models";
import { PRIMARY_ROUTE_IDS, routePath } from "../../routes";
import { AppShell, type PrimaryNavItem } from "./AppShell";

const MainLayout: React.FC = () => {
  useDeviceSync();
  const location = useLocation();
  const navigate = useNavigate();

  const { selectedDeviceId: deviceId } = useStationContext();
  const { data: telemetry } = useDeviceTelemetry(deviceId);
  const { data: systemEvents = [] } = useSystemEvents(deviceId);
  const unreadAlertCount = useMemo(() => {
    if (!systemEvents || !Array.isArray(systemEvents)) return 0;
    return systemEvents.filter((ev: SystemEvent) => {
      const rawTs = ev?.timestamp_ms ?? (ev as any)?.timestamp ?? 0;
      const ts =
        typeof rawTs === "number"
          ? rawTs > 1e12
            ? rawTs
            : rawTs * 1000
          : new Date(rawTs).getTime();
      if (!ts || Number.isNaN(ts)) return false;
      const within24h = Date.now() - ts <= 24 * 60 * 60 * 1000;
      const level = String(ev?.level || "").toLowerCase();
      return (
        within24h &&
        (level === "warning" || level === "critical" || level === "error")
      );
    }).length;
  }, [systemEvents]);

  const navMetadata: Record<
    (typeof PRIMARY_ROUTE_IDS)[number],
    { icon: LucideIcon; label: string; hasBadge?: boolean }
  > = {
    dashboard: { icon: LayoutDashboard, label: "Tổng quan" },
    operations: { icon: SlidersHorizontal, label: "Vận hành" },
    cultivation: { icon: Leaf, label: "Canh tác" },
    journal: {
      icon: AlignLeft,
      label: "Nhật ký",
      hasBadge: unreadAlertCount > 0,
    },
    settings: { icon: Settings, label: "Cài đặt" },
  };
  const navItems: PrimaryNavItem[] = PRIMARY_ROUTE_IDS.map((id) => ({
    id,
    ...navMetadata[id],
    path: routePath(id),
    active:
      matchPath({ path: routePath(id), end: false }, location.pathname) !==
      null,
  }));

  return (
    <AppShell
      navItems={navItems}
      availability={telemetry?.availability}
      deviceId={deviceId}
      onNavigate={navigate}
    >
      <Outlet />
    </AppShell>
  );
};

export default MainLayout;
