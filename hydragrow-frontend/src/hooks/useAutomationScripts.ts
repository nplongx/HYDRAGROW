import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { apiDelete, apiGet, apiPost, apiPut } from '../lib/apiClient';
import type { ApplyTemplateRequest, FlowTemplateOverride, UpsertScriptRequest, UserScript } from '../types/automation';

export function useAutomationScripts(deviceId: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: ['automation-scripts', deviceId],
    queryFn: () =>
      apiGet<{ status: string; data: UserScript[] }>(`/devices/${deviceId}/scripts`).then(
        (r) => r.data,
      ),
    enabled: options?.enabled !== undefined ? options.enabled && !!deviceId : !!deviceId,
  });
}

export function useCreateAutomationScript(deviceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: UpsertScriptRequest) =>
      apiPost<{ status: string; data: UserScript }, UpsertScriptRequest>(`/devices/${deviceId}/scripts`, body),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['automation-scripts', deviceId] }),
  });
}

export function useUpdateAutomationScript(deviceId: string, scriptId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: UpsertScriptRequest) =>
      apiPut<{ status: string; data: UserScript }>(`/devices/${deviceId}/scripts/${scriptId}`, body),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['automation-scripts', deviceId] }),
  });
}

export function useDeleteAutomationScript(deviceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (scriptId: string) =>
      apiDelete<{ status: string }>(`/devices/${deviceId}/scripts/${scriptId}`),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['automation-scripts', deviceId] }),
  });
}

export function useValidateAutomationScript(deviceId: string) {
  return useMutation({
    mutationFn: (body: UpsertScriptRequest) =>
      apiPost<{ valid: boolean; error?: string }, UpsertScriptRequest>(`/devices/${deviceId}/scripts/validate`, body),
  });
}

export function useApplyFlowTemplate(deviceId: string, scriptId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: ApplyTemplateRequest) =>
      apiPost<{ status: string; override_script_ids: string[]; overrides: FlowTemplateOverride[] }, ApplyTemplateRequest>(
        `/devices/${deviceId}/scripts/${scriptId}/apply-template`,
        body,
      ),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['automation-scripts'] }),
  });
}

export function useSyncFlowTemplate(deviceId: string, scriptId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () =>
      apiPost<{ status: string; synced_devices_count: number }, Record<string, never>>(
        `/devices/${deviceId}/scripts/${scriptId}/sync-template`,
        {},
      ),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['automation-scripts'] }),
  });
}
