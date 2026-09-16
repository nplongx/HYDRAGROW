import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { adminApi, type ProvisionUserPayload, type UpdateUserPayload } from '../api/admin';

const adminUsersKey = ['admin-users'] as const;

export function useAdminUsers() {
  const queryClient = useQueryClient();
  const usersQuery = useQuery({
    queryKey: adminUsersKey,
    queryFn: ({ signal }) => adminApi.listUsers(signal),
    retry: false,
  });
  const provisionMutation = useMutation({
    mutationFn: (payload: ProvisionUserPayload) => adminApi.provisionUser(payload),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: adminUsersKey }),
  });
  const updateMutation = useMutation({
    mutationFn: ({ userId, payload }: { userId: number; payload: UpdateUserPayload }) =>
      adminApi.updateUser(userId, payload),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: adminUsersKey }),
  });

  return {
    ...usersQuery,
    users: usersQuery.data ?? [],
    provisionUser: provisionMutation.mutateAsync,
    updateUser: (userId: number, payload: UpdateUserPayload) => updateMutation.mutateAsync({ userId, payload }),
    isProvisioning: provisionMutation.isPending,
    isUpdating: updateMutation.isPending,
  };
}

export function useAdminScopes() {
  return useQuery({
    queryKey: ['admin-scopes'],
    queryFn: () => adminApi.listScopes(),
    retry: false,
  });
}
