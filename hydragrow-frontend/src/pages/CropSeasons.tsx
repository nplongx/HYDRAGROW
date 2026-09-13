import { useState } from 'react';
import { Sprout } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { useCropSeason } from '../hooks/useCropSeason';
import { useDeviceStore } from '../store/useDeviceStore';
import { PageHeader } from '../components/ui/PageHeader';
import { LoadingState } from '../components/ui/LoadingState';
import { ActiveSeasonCard } from '../components/seasons/ActiveSeasonCard';
import { SeasonPhotoJournal } from '../components/seasons/SeasonPhotoJournal';
import { CreateSeasonForm } from '../components/seasons/CreateSeasonForm';
import { SeasonHistoryList } from '../components/seasons/SeasonHistoryList';
import { SeasonCompletionSummary } from '../components/seasons/SeasonCompletionSummary';
import type { CropSeason } from '../types/models';

export const CropSeasons = ({ variant = 'standalone' }: { variant?: 'standalone' | 'embedded' }) => {
  const { activeSeason, history, isLoading, createSeason, endSeason, updateSeason, deleteSeason } = useCropSeason();
  const deviceId = useDeviceStore((s) => s.deviceId);
  const queryClient = useQueryClient();
  const [justEnded, setJustEnded] = useState<{ season: CropSeason; days: number } | null>(null);

  if (isLoading && !activeSeason && history.length === 0) {
    return <LoadingState message="Đang tải danh sách mùa vụ..." />;
  }

  const filteredHistory = history.filter((season) => season.id !== activeSeason?.id);
  const photoCount = justEnded
    ? (queryClient.getQueryData<{ id: string }[]>(['season-photos', deviceId, justEnded.season.id]) ?? []).length
    : 0;

  const contentNode = (
    <>
      {/* Mùa vụ đang chạy HOẶC Form tạo mới */}
      {activeSeason ? (
        <>
          <ActiveSeasonCard
            activeSeason={activeSeason}
            isLoading={isLoading}
            onEndSeason={endSeason}
            onUpdateSeason={updateSeason}
            onEnded={(season, days) => setJustEnded({ season, days })}
          />
          <SeasonPhotoJournal seasonId={activeSeason.id} seasonStartTime={activeSeason.start_time} />
        </>
      ) : (
        <CreateSeasonForm isLoading={isLoading} onCreateSeason={createSeason} />
      )}

      {/* Lịch sử các mùa vụ trước */}
      <SeasonHistoryList seasons={filteredHistory} onDelete={(season) => deleteSeason(season.id)} />

      {justEnded && (
        <SeasonCompletionSummary
          seasonName={justEnded.season.name}
          totalDaysGrown={justEnded.days}
          photoCount={photoCount}
          onClose={() => setJustEnded(null)}
        />
      )}
    </>
  );

  if (variant === 'embedded') return contentNode;

  return (
    <div className="p-4 md:p-8 max-w-4xl mx-auto pb-28">
      {/* Header Trang */}
      <PageHeader
        icon={Sprout}
        title="Quản Lý Mùa Vụ"
        subtitle="Theo dõi và ghi chép chu kỳ sinh trưởng của cây trồng"
      />
      {contentNode}
    </div>
  );
};

export default CropSeasons;
