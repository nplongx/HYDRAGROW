// src/lib/authToken.ts
// Giữ ID token Firebase hiện tại trong bộ nhớ (không phải browser storage).
// AuthContext (src/contexts/AuthContext.tsx) là nơi duy nhất được phép gọi setIdToken.
// httpFetch (src/platform/http.ts) chỉ đọc qua getIdToken.

let currentIdToken: string | null = null;

export function setIdToken(token: string | null): void {
  currentIdToken = token;
}

export function getIdToken(): string | null {
  return currentIdToken;
}
