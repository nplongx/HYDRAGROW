import { apiPost } from '../lib/apiClient';

export const notificationsApi = {
  registerFcmToken: (fcmToken: string, deviceId: string) =>
    apiPost('/notifications/register', { fcm_token: fcmToken, device_id: deviceId }),
};
