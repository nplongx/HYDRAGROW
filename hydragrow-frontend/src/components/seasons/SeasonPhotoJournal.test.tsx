import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { SeasonPhotoJournal } from './SeasonPhotoJournal';
import { useDeviceStore } from '../../store/useDeviceStore';

const mockPhotos = [
    { id: 'p1', day_offset: 4, cloudinary_url: 'https://res.cloudinary.com/demo/p1.jpg' },
    { id: 'p2', day_offset: 11, cloudinary_url: 'https://res.cloudinary.com/demo/p2.jpg' },
];

vi.mock('@tanstack/react-query', async (importOriginal) => {
    const actual = await importOriginal<typeof import('@tanstack/react-query')>();
    return {
        ...actual,
        useQuery: () => ({ data: mockPhotos, refetch: vi.fn() }),
    };
});

describe('SeasonPhotoJournal', () => {
    let queryClient: QueryClient;

    beforeEach(() => {
        queryClient = new QueryClient();
        useDeviceStore.setState({
            deviceId: 'dev-1',
            settings: { backend_url: 'http://test', api_key: 'k' },
        } as any);
    });

    const renderWithClient = (ui: React.ReactNode) =>
        render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);

    it('hiển thị đúng nhãn "Ngày N" cho từng ảnh', () => {
        renderWithClient(<SeasonPhotoJournal seasonId="season-1" seasonStartTime={new Date().toISOString()} />);
        expect(screen.getByText('Ngày 4')).toBeInTheDocument();
        expect(screen.getByText('Ngày 11')).toBeInTheDocument();
    });

    it('hiển thị ô "+ Thêm" luôn ở cuối', () => {
        renderWithClient(<SeasonPhotoJournal seasonId="season-1" seasonStartTime={new Date().toISOString()} />);
        expect(screen.getByText('+ Thêm')).toBeInTheDocument();
    });
});
