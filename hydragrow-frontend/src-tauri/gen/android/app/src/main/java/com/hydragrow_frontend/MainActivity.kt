package com.hydragrow_frontend

import android.os.Bundle
import android.util.Log
import com.google.firebase.FirebaseApp // 🟢 1. THÊM IMPORT NÀY
import com.google.firebase.messaging.FirebaseMessaging
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import kotlin.concurrent.thread

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // 🟢 2. ÉP FIREBASE KHỞI TẠO TRƯỚC KHI LÀM BẤT CỨ VIỆC GÌ KHÁC
        FirebaseApp.initializeApp(this)

        // 1. Lấy FCM Token từ Google Play Services
        FirebaseMessaging.getInstance().token.addOnCompleteListener { task ->
            if (!task.isSuccessful) {
                Log.w("FCM_NATIVE", "Không thể lấy FCM token", task.exception)
                return@addOnCompleteListener
            }

            val token = task.result ?: return@addOnCompleteListener
            Log.d("FCM_NATIVE", "📱 Đã lấy được Token: $token")

            // 2. Gửi thẳng Token lên Backend Actix bằng Kotlin (Chạy luồng ngầm)
            thread {
                try {
                    val url = URL("https://hydragrow.onrender.com/api/notifications/register")
                    val conn = url.openConnection() as HttpURLConnection
                    conn.requestMethod = "POST"
                    conn.setRequestProperty("Content-Type", "application/json")
                    conn.setRequestProperty("Accept", "application/json")
                    conn.setRequestProperty("x-api-key", "long")

                    conn.doOutput = true

                    // Đóng gói JSON { "fcm_token": "..." }
                    val jsonParam = JSONObject()
                    jsonParam.put("fcm_token", token)

                    conn.outputStream.use { os ->
                        val input = jsonParam.toString().toByteArray(Charsets.UTF_8)
                        os.write(input, 0, input.size)
                    }

                    Log.d("FCM_NATIVE", "✅ Đã đăng ký Token! HTTP Status: ${conn.responseCode}")
                    conn.disconnect()
                } catch (e: Exception) {
                    Log.e("FCM_NATIVE", "❌ Lỗi gửi token lên Backend", e)
                }
            }
        }
    }
}
