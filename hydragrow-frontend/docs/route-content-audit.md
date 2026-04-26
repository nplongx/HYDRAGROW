# Route/Page Audit (UI Copy Refresh)

## 1) Danh sách route/page từ cấu hình router

Nguồn: `src/App.tsx`

| Route | Page/Điều hướng | Component render |
|---|---|---|
| `/` | Redirect | `Navigate` -> `/dashboard` |
| `/dashboard` | Dashboard | `Dashboard` |
| `/control` | Control panel | `ControlPanel` |
| `/analytics` | Analytics | `Analytics` |
| `/blockchain` | Blockchain history | `BlockchainHistory` |
| `/crop-seasons` | Crop seasons | `CropSeasons` |
| `/settings` | Settings | `Settings` |
| `/logs` | System log | `SystemLog` |

## 2) Bảng audit copy/UI

> Mục tiêu: viết lại text theo hướng rõ nghĩa, giảm “lòe loẹt” ngôn từ, nhưng **không chạm logic nghiệp vụ**.

| Page | Component chính | Text cần viết lại | Mức độ lòe loẹt | Mức ưu tiên |
|---|---|---|---|---|
| `/dashboard` | `Dashboard` | Hero title/subtitle dạng marketing, trạng thái có icon/gradient quá nổi; chuyển sang câu mô tả ngắn, trực tiếp theo dữ liệu vận hành. | Cao | P1 |
| `/control` | `ControlPanel` | Cụm “Trợ lý châm thông minh (Bán thủ công)”, nhãn nút và helper text mang tính quảng bá; chuẩn hóa sang hướng dẫn thao tác. | Cao | P1 |
| `/analytics` | `Analytics` | Nhãn chart/section viết in hoa + hiệu ứng biểu tượng “⚡ THỜI GIAN THỰC”; thay bằng copy trung tính, nhất quán đơn vị đo. | Cao | P1 |
| `/blockchain` | `BlockchainHistory` | Loading/empty/error message còn cảm tính; chuẩn hóa thông điệp trạng thái + hành động tiếp theo cho user. | Trung bình | P2 |
| `/crop-seasons` | `CropSeasons` | Tiêu đề/nhãn trạng thái (“Active”) trộn ngôn ngữ, nhiều phong cách; thống nhất thuật ngữ tiếng Việt theo nghiệp vụ mùa vụ. | Trung bình | P2 |
| `/logs` | `SystemLog` | Nhãn filter và tiêu đề đã rõ nhưng tone chưa đồng nhất; chuẩn hóa phân loại log theo tác vụ/kết quả. | Thấp-Trung bình | P3 |
| `/settings` | `Settings` | Ưu tiên rà soát nhóm label/description cấu hình để rõ điều kiện áp dụng; giảm câu dài khó scan. | Thấp | P4 |

## 3) Checklist “KHÔNG ĐỤNG LOGIC” (bắt buộc khi review PR)

- [ ] Không đổi hàm xử lý nghiệp vụ.
- [ ] Không đổi schema/validator.
- [ ] Không đổi API contract, query key, mutation flow.
- [ ] Không đổi điều kiện phân quyền/ẩn hiện nghiệp vụ.
- [ ] Chỉ thay đổi text hiển thị, label, helper text, heading, placeholder, tooltip, empty/loading/error copy.
- [ ] Không thay đổi kiểu dữ liệu, thứ tự gọi API, side effect, state machine/fsm transition.
- [ ] Nếu bắt buộc đổi key i18n/copy constant: mapping 1-1, không đổi ý nghĩa domain.

## 4) Thứ tự rollout đề xuất

1. **Trang công khai** (nếu có)  
   Lý do: tác động branding ngay, rủi ro nghiệp vụ thấp.
2. **Dashboard chính**  
   Lý do: traffic cao, cải thiện khả năng đọc trạng thái vận hành nhanh nhất.
3. **Form CRUD**  
   Lý do: giảm sai thao tác nhập liệu nhờ copy rõ ràng.
4. **Trang cài đặt**  
   Lý do: ít tần suất truy cập hơn, cần kiểm tra kỹ cảnh báo/điều kiện áp dụng.

## Gợi ý vận hành PR copy-only

- Mỗi PR chỉ 1 nhóm route theo rollout ở trên.
- Gắn nhãn: `copy-only`, `no-logic-change`.
- Reviewer bắt buộc tick đủ checklist mục (3) trước khi merge.
