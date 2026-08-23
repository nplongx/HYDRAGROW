import { RefreshCw, Wifi, WifiOff, ChevronRight, Cpu } from 'lucide-react';
import { useFleetStatus } from '../hooks/useFleetStatus';
import { useDeviceStore } from '../store/useDeviceStore';
import { useNavigate } from 'react-router-dom';

export function FleetView() {
  const { devices, loading, error, refresh } = useFleetStatus();
  const setDeviceId = useDeviceStore((s) => s.setDeviceId);
  const navigate = useNavigate();

  function selectDevice(deviceId: string) {
    setDeviceId(deviceId);
    navigate('/');
  }

  return (
    <div className="max-w-3xl mx-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Tổng Quan Thiết Bị</h1>
        <button
          onClick={refresh}
          disabled={loading}
          className="flex items-center gap-2 px-3 py-1.5 border rounded-lg text-sm text-gray-600 hover:bg-gray-50"
        >
          <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
          Làm mới
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-50 text-red-700 rounded-lg text-sm">{error}</div>
      )}

      {loading && devices.length === 0 ? (
        <div className="space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-20 bg-gray-100 rounded-xl animate-pulse" />
          ))}
        </div>
      ) : (
        <div className="space-y-3">
          {devices.length === 0 ? (
            <div className="text-center py-12 text-gray-400">
              <Cpu className="mx-auto mb-3" size={40} />
              <p>Chưa có thiết bị nào được liên kết.</p>
              <p className="text-sm mt-1">Vào "Thiết Bị Của Tôi" để thêm thiết bị.</p>
            </div>
          ) : (
            devices.map((d) => (
              <button
                key={d.device_id}
                onClick={() => selectDevice(d.device_id)}
                className="w-full flex items-center gap-4 p-4 border rounded-xl hover:bg-gray-50 transition text-left"
              >
                {/* Online indicator */}
                <div className={`w-10 h-10 rounded-full flex items-center justify-center flex-shrink-0 ${
                  d.is_online ? 'bg-green-100' : 'bg-gray-100'
                }`}>
                  {d.is_online
                    ? <Wifi size={18} className="text-green-600" />
                    : <WifiOff size={18} className="text-gray-400" />
                  }
                </div>

                {/* Device info */}
                <div className="flex-1 min-w-0">
                  <p className="font-medium truncate">{d.label ?? d.device_id}</p>
                  <p className="text-xs text-gray-400">{d.device_id}</p>
                  {d.firmware_version && (
                    <p className="text-xs text-gray-400">FW: {d.firmware_version}</p>
                  )}
                </div>

                {/* Status badge + arrow */}
                <div className="flex items-center gap-2 flex-shrink-0">
                  <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                    d.is_online
                      ? 'bg-green-100 text-green-700'
                      : 'bg-gray-100 text-gray-500'
                  }`}>
                    {d.is_online ? 'Online' : 'Offline'}
                  </span>
                  {d.last_seen && !d.is_online && (
                    <span className="text-xs text-gray-400">
                      {new Date(d.last_seen).toLocaleString('vi-VN', { dateStyle: 'short', timeStyle: 'short' })}
                    </span>
                  )}
                  <ChevronRight size={16} className="text-gray-300" />
                </div>
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}
