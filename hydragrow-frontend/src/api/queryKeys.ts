export const queryKeys = {
  devices: () => ["devices"] as const,
  device: (deviceId: string) => ["device", deviceId] as const,
  config: (deviceId: string) => ["device-config", deviceId] as const,
  telemetry: (deviceId: string, range?: unknown) =>
    [
      "device-telemetry",
      deviceId,
      ...(range === undefined ? [] : [range]),
    ] as const,
  analyticsHealth: (deviceId: string) => ["device-health", deviceId] as const,
  controlCommands: (deviceId: string) =>
    ["control-commands", deviceId] as const,
  seasons: (deviceId: string) => ["seasons", deviceId] as const,
  seasonActive: (deviceId: string) => ["seasons", deviceId, "active"] as const,
  seasonHistory: (deviceId: string) =>
    ["seasons", deviceId, "history"] as const,
  recipes: () => ["recipes"] as const,
  recipeStatus: (deviceId: string) =>
    ["device", deviceId, "recipe-status"] as const,
} as const;
