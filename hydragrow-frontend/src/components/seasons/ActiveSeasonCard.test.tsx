import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ActiveSeasonCard } from './ActiveSeasonCard';
import type { CropSeason } from '../../types/models';

vi.mock('../../hooks/useActiveRecipeStatus', () => ({
    useActiveRecipeStatus: () => ({
        activeRecipe: {
            recipe_id: 'Công thức Xà lách',
            current_stage_index: 1,
            stages: [
                { name: 'Ươm mầm', duration_sec: 10 * 86400 },
                { name: 'Sinh trưởng', duration_sec: 15 * 86400 },
                { name: 'Ra hoa', duration_sec: 10 * 86400 },
            ],
        },
        currentStage: { name: 'Sinh trưởng' },
        isError: false,
    }),
}));

const season: CropSeason = {
    id: 's1',
    device_id: 'd1',
    name: 'Vụ 1',
    plant_type: 'Xà lách',
    description: null,
    start_time: new Date(Date.now() - 28 * 86400000).toISOString(),
    end_time: null,
    status: 'active',
};

describe('ActiveSeasonCard — tiến độ & cảnh báo chậm', () => {
    it('hiện cảnh báo chậm tiến độ khi đã quá hạn giai đoạn hiện tại', () => {
        render(<ActiveSeasonCard activeSeason={season} isLoading={false} onEndSeason={vi.fn()} />);
        expect(screen.getByText(/Chậm hơn dự kiến/)).toBeInTheDocument();
    });

    it('hiện đúng số ngày đã trôi qua trên tổng số ngày dự kiến', () => {
        render(<ActiveSeasonCard activeSeason={season} isLoading={false} onEndSeason={vi.fn()} />);
        expect(screen.getByText(/Ngày 28\/35/)).toBeInTheDocument();
    });
});
