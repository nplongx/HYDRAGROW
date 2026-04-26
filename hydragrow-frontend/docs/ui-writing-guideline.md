# UI Writing Guideline

## Mục tiêu
Viết ngắn, rõ và giúp người dùng biết bước tiếp theo ngay khi đọc.

## 1) Nguyên tắc ngôn từ
- **Ngắn:** Ưu tiên câu 1 ý, bỏ từ thừa.
- **Chủ động:** Dùng động từ rõ hành động ("Lưu cài đặt", "Thử lại").
- **Tích cực:** Tránh đổ lỗi, tập trung vào cách xử lý.
- **Dễ hiểu:** Tránh thuật ngữ nội bộ; nếu bắt buộc, kèm giải thích ngắn.

## 2) Quy ước xưng hô
- Dùng nhất quán đại từ **"Bạn"** trong toàn bộ sản phẩm.
- Tránh đổi qua lại giữa "bạn", "người dùng", "admin" trong cùng ngữ cảnh.

## 3) Mẫu câu theo trạng thái

### Loading
- "Đang tải dữ liệu..."
- "Đang đồng bộ cài đặt..."
- "Vui lòng chờ trong giây lát."

### Empty
- "Chưa có dữ liệu."
- "Bạn chưa tạo mùa vụ nào."
- "Hãy thêm dữ liệu để bắt đầu."

### Success
- "Đã lưu cài đặt."
- "Đã cập nhật thành công."
- "Đã ghi nhận dữ liệu."

### Error (có hướng dẫn hành động)
- Công thức khuyến nghị: **[Vấn đề] + [Hành động ngay] + [Bước tiếp theo]**.
- Ví dụ:
  - "Không thể kết nối máy chủ. Vui lòng thử lại. Nếu vẫn lỗi, kiểm tra URL máy chủ và API key."
  - "Không thể lưu cấu hình. Vui lòng kiểm tra các trường đang báo lỗi rồi thử lại."
  - "Phiên làm việc đã hết hạn. Vui lòng đăng nhập lại để tiếp tục."

## 4) Quy tắc cho button/title/helper/toast/modal
- **Button:** Bắt đầu bằng động từ, tối đa 2-4 từ.
- **Title:** Mô tả đúng nội dung, không viết hoa toàn bộ.
- **Helper text:** Nêu mục đích + giới hạn nhập liệu (nếu có).
- **Toast:** Một thông điệp chính; lỗi phải có hướng dẫn hành động.
- **Modal confirm:**
  - Tiêu đề ngắn: "Xác nhận kết thúc mùa vụ?"
  - Nội dung nêu hệ quả: "Sau khi kết thúc, bạn không thể chỉnh sửa mùa vụ này."
  - Nút hành động rõ ràng: "Xác nhận", "Hủy".

## 5) Checklist trước khi merge
- [ ] Không dùng ALL CAPS cho nội dung chính.
- [ ] Cùng một hành động chỉ dùng một thuật ngữ.
- [ ] Thông báo lỗi luôn có "Vui lòng thử lại" và ít nhất một gợi ý bước tiếp theo.
- [ ] Xưng hô "Bạn" nhất quán trên toàn bộ màn hình.
