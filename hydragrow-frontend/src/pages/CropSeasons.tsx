import { Sprout } from 'lucide-react';
import { useCropSeason } from '../hooks/useCropSeason';
import { PageHeader } from '../components/ui/PageHeader';
import { LoadingState } from '../components/ui/LoadingState';
import { ActiveSeasonCard } from '../components/seasons/ActiveSeasonCard';
import { CreateSeasonForm } from '../components/seasons/CreateSeasonForm';
import { SeasonHistoryList } from '../components/seasons/SeasonHistoryList';

export const CropSeasons = ({ variant = 'standalone' }: { variant?: 'standalone' | 'embedded' }) => {
  const { activeSeason, history, isLoading, createSeason, endSeason, updateSeason } = useCropSeason();

  if (isLoading && !activeSeason && history.length === 0) {
    return <LoadingState message="Đang tải danh sách mùa vụ..." />;
  }

  const filteredHistory = history.filter((season) => season.id !== activeSeason?.id);


  const contentNode = (
    <>
      {/* Mùa vụ đang chạy HOẶC Form tạo mới */}
      {activeSeason ? (
        <ActiveSeasonCard
          activeSeason={activeSeason}
          isLoading={isLoading}
          onEndSeason={endSeason}
          onUpdateSeason={updateSeason}
        />
      ) : (
        <CreateSeasonForm isLoading={isLoading} onCreateSeason={createSeason} />
      )}

      {/* Lịch sử các mùa vụ trước */}
      <SeasonHistoryList seasons={filteredHistory} />
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
