// src/lib/firebaseAuth.ts
// Bọc firebase/auth: đăng nhập bằng tài khoản được cấp sẵn (không có đăng ký).

import {
  getAuth,
  signInWithEmailAndPassword,
  signOut as firebaseSignOut,
  onAuthStateChanged,
  onIdTokenChanged,
  type User,
} from 'firebase/auth';
import { app } from './firebase';
import { setIdToken } from './authToken';

export const auth = getAuth(app);

/** Đăng nhập bằng email/password của tài khoản admin đã cấp sẵn. */
export async function loginWithEmailPassword(email: string, password: string): Promise<User> {
  const credential = await signInWithEmailAndPassword(auth, email.trim(), password);
  return credential.user;
}

export async function logout(): Promise<void> {
  await firebaseSignOut(auth);
  setIdToken(null);
}

/** Theo dõi trạng thái đăng nhập (đăng nhập/đăng xuất). */
export function subscribeAuthState(callback: (user: User | null) => void): () => void {
  return onAuthStateChanged(auth, callback);
}

/**
 * Theo dõi ID token: Firebase SDK tự làm mới token trước khi hết hạn (~1h)
 * và bắn lại callback này, nên không cần tự đặt timer refresh thủ công.
 */
export function subscribeIdToken(callback: (token: string | null) => void): () => void {
  return onIdTokenChanged(auth, async (user) => {
    if (!user) {
      callback(null);
      return;
    }
    const token = await user.getIdToken();
    callback(token);
  });
}

/** Map mã lỗi Firebase sang thông báo tiếng Việt dễ hiểu cho người dùng. */
export function describeAuthError(code: string): string {
  switch (code) {
    case 'auth/invalid-credential':
    case 'auth/wrong-password':
    case 'auth/user-not-found':
      return 'Email hoặc mật khẩu không đúng.';
    case 'auth/too-many-requests':
      return 'Đã thử sai quá nhiều lần. Vui lòng thử lại sau ít phút.';
    case 'auth/user-disabled':
      return 'Tài khoản này đã bị vô hiệu hoá.';
    case 'auth/network-request-failed':
      return 'Lỗi mạng, vui lòng kiểm tra kết nối internet.';
    default:
      return 'Đăng nhập thất bại. Vui lòng thử lại.';
  }
}
