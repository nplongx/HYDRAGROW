import { useRef } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Plus, Trash2 } from 'lucide-react';
import toast from 'react-hot-toast';
import { httpFetch } from '../../platform/http';
import { useDeviceStore } from '../../store/useDeviceStore';

interface SeasonPhoto {
    id: string;
    day_offset: number;
    cloudinary_url: string;
}

interface SeasonPhotoJournalProps {
    seasonId: string;
    seasonStartTime: string;
}

export const SeasonPhotoJournal = ({ seasonId, seasonStartTime }: SeasonPhotoJournalProps) => {
    const deviceId = useDeviceStore((s) => s.deviceId);
    const settings = useDeviceStore((s) => s.settings);
    const queryClient = useQueryClient();
    const fileInputRef = useRef<HTMLInputElement>(null);

    const headers = { 'Content-Type': 'application/json', 'X-API-Key': settings?.api_key || '' };
    const baseUrl = `${settings?.backend_url}/api/devices/${deviceId}/seasons/${seasonId}/photos`;

    const { data: photos = [] } = useQuery<SeasonPhoto[]>({
        queryKey: ['season-photos', deviceId, seasonId],
        enabled: Boolean(deviceId && settings?.backend_url),
        queryFn: async () => {
            const res = await httpFetch(baseUrl, { headers });
            if (!res.ok) return [];
            const json = await res.json();
            return json.data || [];
        },
    });

    const handleFileSelected = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        e.target.value = '';
        if (!file) return;

        try {
            const signRes = await httpFetch(`${baseUrl}/sign`, { method: 'POST', headers });
            if (!signRes.ok) throw new Error('Không lấy được chữ ký upload');
            const { data: sign } = await signRes.json();

            const form = new FormData();
            form.append('file', file);
            form.append('api_key', sign.api_key);
            form.append('timestamp', String(sign.timestamp));
            form.append('signature', sign.signature);
            form.append('folder', sign.folder);
            form.append('signature_algorithm', sign.signature_algorithm);

            const uploadRes = await fetch(`https://api.cloudinary.com/v1_1/${sign.cloud_name}/image/upload`, {
                method: 'POST',
                body: form,
            });
            if (!uploadRes.ok) throw new Error('Upload lên Cloudinary thất bại');
            const uploaded = await uploadRes.json();

            const dayOffset = Math.max(
                0,
                Math.floor((Date.now() - new Date(seasonStartTime).getTime()) / 86400000),
            );

            const saveRes = await httpFetch(baseUrl, {
                method: 'POST',
                headers,
                body: JSON.stringify({
                    cloudinary_public_id: uploaded.public_id,
                    cloudinary_url: uploaded.secure_url,
                    day_offset: dayOffset,
                }),
            });
            if (!saveRes.ok) throw new Error('Không lưu được ảnh vào hệ thống');
            toast.success('Đã thêm ảnh vào nhật ký.');
            queryClient.invalidateQueries({ queryKey: ['season-photos', deviceId, seasonId] });
        } catch (err) {
            toast.error(err instanceof Error ? err.message : 'Lỗi khi thêm ảnh');
        }
    };

    const handleDeletePhoto = async (photo: SeasonPhoto) => {
        if (!window.confirm(`Xoá ảnh "Ngày ${photo.day_offset}" khỏi nhật ký?`)) return;
        try {
            const res = await httpFetch(`${baseUrl}/${photo.id}`, { method: 'DELETE', headers });
            if (!res.ok) throw new Error('Không xoá được ảnh');
            toast.success('Đã xoá ảnh.');
            queryClient.invalidateQueries({ queryKey: ['season-photos', deviceId, seasonId] });
        } catch (err) {
            toast.error(err instanceof Error ? err.message : 'Lỗi khi xoá ảnh');
        }
    };

    return (
        <div className="space-y-2">
            <p className="text-[10px] font-bold uppercase tracking-wider text-faint">Nhật ký ảnh</p>
            <div className="flex gap-3 overflow-x-auto pb-1">
                {photos.map((photo) => (
                    <div key={photo.id} className="shrink-0 w-20 text-center space-y-1 group">
                        <div className="relative">
                            <img
                                src={photo.cloudinary_url}
                                alt={`Ngày ${photo.day_offset}`}
                                className="w-20 h-20 object-cover rounded-xl border border-line"
                            />
                            <button
                                type="button"
                                aria-label={`Xoá ảnh ngày ${photo.day_offset}`}
                                onClick={() => handleDeletePhoto(photo)}
                                className="absolute -top-1.5 -right-1.5 p-1 bg-white border border-line rounded-full text-faint hover:text-red-600 hover:border-red-200 hidden group-hover:flex items-center justify-center shadow-sm"
                            >
                                <Trash2 size={12} />
                            </button>
                        </div>
                        <p className="text-[10px] font-semibold text-text-muted">Ngày {photo.day_offset}</p>
                    </div>
                ))}
                <button
                    onClick={() => fileInputRef.current?.click()}
                    className="shrink-0 w-20 h-20 flex flex-col items-center justify-center gap-1 rounded-xl border-2 border-dashed border-line text-primary hover:bg-soft transition-colors"
                >
                    <Plus size={18} />
                    <span className="text-[10px] font-bold">+ Thêm</span>
                </button>
                <input ref={fileInputRef} type="file" accept="image/*" className="hidden" onChange={handleFileSelected} />
            </div>
        </div>
    );
};
