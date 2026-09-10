// src/lib/firebaseAuth.ts
// Bọc firebase/auth: đăng nhập, tự đăng ký, đăng nhập Google, quên mật khẩu.

import {
  getAuth,
  signInWithEmailAndPassword,
  createUserWithEmailAndPassword,
  sendPasswordResetEmail,
  signInWithPopup,
  GoogleAuthProvider,
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

/** Tự đăng ký tài khoản mới (backend tự cấp scope đọc mặc định ở lần truy cập đầu). */
export async function registerWithEmailPassword(email: string, password: string): Promise<User> {
  const credential = await createUserWithEmailAndPassword(auth, email.trim(), password);
  return credential.user;
}

/** Đăng nhập bằng Google (popup). */
export async function signInWithGoogle(): Promise<User> {
  const credential = await signInWithPopup(auth, new GoogleAuthProvider());
  return credential.user;
}

/** Gửi email đặt lại mật khẩu qua Firebase. */
export async function sendPasswordReset(email: string): Promise<void> {
  await sendPasswordResetEmail(auth, email.trim());
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
    case 'auth/email-already-in-use':
      return 'Email này đã được đăng ký. Vui lòng đăng nhập hoặc dùng email khác.';
    case 'auth/invalid-email':
      return 'Địa chỉ email không hợp lệ.';
    case 'auth/weak-password':
      return 'Mật khẩu quá yếu (tối thiểu 6 ký tự).';
    case 'auth/operation-not-allowed':
      return 'Đăng ký bằng email hiện đang bị tắt ở cấu hình Firebase.';
    case 'auth/popup-closed-by-user':
    case 'auth/cancelled-popup-request':
      return 'Đã huỷ cửa sổ đăng nhập Google.';
    case 'auth/account-exists-with-different-credential':
      return 'Email đã tồn tại ở phương thức đăng nhập khác. Hãy đăng nhập bằng email/mật khẩu.';
    case 'auth/missing-email':
    case 'auth/missing-password':
      return 'Vui lòng nhập đầy đủ email và mật khẩu.';
    default:
      return 'Đăng nhập thất bại. Vui lòng thử lại.';
  }
}

/** Trả về 1 nếu lỗi thuộc về field email, 2 nếu thuộc về mật khẩu, 0 nếu không xác định. */
export function authErrorField(code: string): 0 | 1 | 2 {
  switch (code) {
    case 'auth/invalid-email':
    case 'auth/user-not-found':
    case 'auth/email-already-in-use':
    case 'auth/missing-email':
      return 1;
    case 'auth/wrong-password':
    case 'auth/weak-password':
    case 'auth/missing-password':
    case 'auth/invalid-credential':
      return 2;
    default:
      return 0;
  }
}
