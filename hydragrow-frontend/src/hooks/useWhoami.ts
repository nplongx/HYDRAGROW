import { useQuery } from '@tanstack/react-query';
import { apiGet } from '../lib/apiClient';
import { useAuth } from '../contexts/AuthContext';

export interface WhoamiUser {
  id: number;
  firebase_uid: string;
  email: string;
  display_name?: string | null;
  role?: 'admin' | 'operator' | 'viewer' | null;
  scopes: string[];
  is_active: boolean;
  preferences?: Record<string, any> | null;
}

export function useWhoami() {
  const { user } = useAuth();
  return useQuery({
    queryKey: ['whoami', user?.uid],
    queryFn: async () => {
      const res = await apiGet<{ status: string; data: WhoamiUser }>('/admin/whoami');
      return res.data;
    },
    enabled: !!user,
    staleTime: 60_000,
    retry: false,
  });
}
