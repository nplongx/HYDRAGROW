# Khung Ra Quyết Định Q1: Định Vị Hộ Gia Đình vs Trang Trại Thương Mại (Q1 Decision Framework)

**Mã tài liệu:** `DOCS-DS-L3-Q1-FRAMEWORK-001`  
**Trạng thái:** `ĐÃ KHÓA / CHỜ TÍN HIỆU THỰC NGHIỆM (BLOCKED on External Signal)`  
**Ngày cập nhật:** 2026-09-13  
**Tác giả:** Track E Lead (HydraGrow Core Team)  
**Kỹ năng thiết kế áp dụng:** `design-rationale`, `design-principles`, `stakeholder-alignment`, `business-design`

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

### 1.1 Tình Trạng Hiện Tại Của Câu Hỏi Q1
Trong đặc tả hệ thống (`docs/hydragrow-hifi-spec.md`, dòng 117–118), câu hỏi cốt lõi số 1 được đặt ra:
> *"Fleet = 1-station household vs multi-station commercial priority?"* (Ưu tiên sản phẩm là hệ thống 1 trạm cho hộ gia đình hay cụm nhiều trạm cho trang trại thương mại?)

Hiện tại, câu hỏi này **bị chặn (BLOCKED)** vì chưa thể tiến hành phỏng vấn sâu trực tiếp với khách hàng mục tiêu do giới hạn tiếp cận hiện trường và dữ liệu sản xuất thực tế chưa được truy vấn (`docs/discovery/2026-09-12-user-discovery.md` §9, Bảng trạng thái thẩm quyền, dòng 671).

### 1.2 Tái Định Hình Câu Hỏi Q1 Theo Cấu Trúc Đa Tầng (§11.4)
Tài liệu nghiên cứu người dùng (`docs/discovery/2026-09-12-user-discovery.md` §11.4, dòng 746–753) đã chứng minh rằng cách đặt câu hỏi nhị phân **"Hộ gia đình HOẶC Thương mại"** là một sai lầm về mặt chiến lược sản phẩm:
- Không có bất kỳ đối thủ cạnh tranh nào trên thị trường phục vụ cả hai nhóm đối tượng này bằng **một giao diện phẳng, không phân biệt**.
- Người dùng hộ gia đình (Minh - Balcony Hobbyist) cần sự an tâm, không quan tâm và không muốn thấy ma trận phân quyền hay mã UID phức tạp.
- Người dùng thương mại (Anh Thắng - Commercial Farm Manager) đòi hỏi phân quyền theo tổ chức, chia sẻ nhà màng, kiểm soát rủi ro thao tác và phân bổ trạm theo vùng canh tác.
- Đối thủ duy nhất bao phủ cả hai thị trường thành công là **Growlink** làm được điều đó nhờ **chiến lược phân tầng rõ ràng (Tiered Product Architecture)**: tầng miễn phí / $25/tháng chỉ cho phép 1 người dùng và không có phân quyền; tầng thương mại ($1,000/tháng) mở khóa tối đa 20 người dùng cùng tính năng quản trị phân quyền theo trạm.

**Đề xuất chiến lược:** Thay vì chọn 1 trong 2, HydraGrow định vị sản phẩm theo mô hình phân tầng trên nền tảng kỹ thuật dùng chung:
1. Giữ vững lõi backend, mô hình dữ liệu InfluxDB/PostgreSQL và giao thức MQTT vốn đã hỗ trợ đa trạm (`device_ownership`, `/api/fleet/summary`).
2. Thiết kế giao diện phía trước (Frontend) mang tính phòng vệ (defensive design), tích hợp sẵn các điểm mở rộng (extension points) cho phép kích hoạt trải nghiệm thương mại mà không làm tăng tải nhận thức (cognitive overload) của người dùng hộ gia đình.

---

## 2. Kiểm Kê Bằng Chứng (Evidence Inventory)

Toàn bộ các phân tích dưới đây được trích xuất từ dữ liệu mã nguồn và tài liệu nghiên cứu đã được kiểm chứng trong kho lưu trữ:

| Hướng Lựa Chọn | Bằng Chứng Ủng Hộ (Pros) | Bằng Chứng Phản Đối / Rủi Ro (Cons) | Nguồn Trích Dẫn Cụ Thể |
|---|---|---|---|
| **Tùy Chọn A: Ưu Tiên Hộ Gia Đình (Hobbyist-First)** | - Nhu cầu lớn tại các đô thị; người dùng cần sự đơn giản "cắm và chạy" (plug-and-play).<br>- Phù hợp với kiến trúc điều hướng hiện tại của Frontend: `MainLayout.tsx` và `useDeviceStore.ts` đang neo chặt vào phạm vi 1 trạm duy nhất.<br>- Tránh được sự phức tạp về mặt pháp lý và cam kết SLA thương mại khắt khe. | - Khách hàng hộ gia đình có mức sẵn sàng chi trả (willingness-to-pay) thấp ($0 - $25/tháng, xem Growlink §11.2).<br>- Thị trường thiết bị tiêu dùng biến động mạnh (bài học AeroGarden đóng cửa rồi tái khởi động 2024–2026, §11.1 dòng 696).<br>- Hệ thống IoT tự vận hành (MQTT, PostgreSQL, InfluxDB) quá phức tạp cho người dùng thông thường nếu không có giải pháp đóng gói dạng hộp phần cứng khép kín (§11.1 dòng 702). | `docs/discovery/2026-09-12-user-discovery.md` §4.1 (dòng 220), §5 (Persona Minh dòng 291), §11.1 (dòng 691–706). `hydragrow-frontend/src/components/layout/MainLayout.tsx` (dòng 34–40). |
| **Tùy Chọn B: Ưu Tiên Thương Mại (Commercial-First)** | - Giá trị hợp đồng phần mềm/phần cứng cao ($50 - $1,000/tháng/cơ sở canh tác, xem Autogrow và Growlink §11.2).<br>- Kiến trúc backend đã hỗ trợ sẵn quan hệ đa trạm: bảng `device_ownership` có khóa `UNIQUE(user_id, device_id)` và API `/api/fleet/summary` đã hoạt động.<br>- Nhu cầu kiểm soát an toàn chất dinh dưỡng để bảo vệ mùa vụ trăm triệu đồng là có thật (§4.2 Persona Anh Thắng). | - Đòi hỏi hệ thống phân quyền đa chiều (Priva có phân quyền theo ứng dụng, Argus Axia có RBAC phân cấp theo tổ chức, §11.2 dòng 715–716) — mô hình 3 vai trò phẳng hiện tại của HydraGrow chưa đủ đáp ứng.<br>- Thiếu tính năng chia sẻ nhà trạm liên tổ chức (multi-tenant site sharing).<br>- Lỗi bảo mật quyền truy cập trước đây (Discrepancy 1 & 2 trong §9 dòng 672) sẽ làm mất uy tín trước khách hàng doanh nghiệp nếu tái diễn. | `docs/discovery/2026-09-12-user-discovery.md` §4.2 (dòng 250), §5 (Persona Anh Thắng dòng 333), §11.2 (dòng 709–725). `hydragrow-backend/migrations/20260823100000_device_ownership.sql` (dòng 8). `hydragrow-backend/src/api/fleet.rs` (dòng 113). |
| **Tùy Chọn C: Kiến Trúc Phân Tầng (Tiered Product Architecture - Đề Xuất)** | - Học tập thành công duy nhất của Growlink (§11.4 dòng 749): bảo vệ người dùng gia đình khỏi sự rối rắm, đồng thời mở khóa tính năng thương mại cho khách hàng trả phí.<br>- Giữ nguyên 100% mã nguồn backend, tránh phân nhánh hai ứng dụng riêng biệt.<br>- Các điểm mở rộng Layer 3 (Tracks A-D) đã được thiết kế sẵn để thích ứng mà không cần viết lại mã. | - Đòi hỏi hệ thống quản lý cờ tính năng (Feature Flagging) hoặc bảng đăng ký gói người dùng (`subscription_tier`) trong backend.<br>- Cần bảo trì hai luồng trải nghiệm (Onboarding & Phân quyền) trong cùng một cơ sở mã giao diện. | `docs/discovery/2026-09-12-user-discovery.md` §11.4 (dòng 746–753). `docs/design-system/layer3/PERMISSION-MODEL-SPEC.md` §6 (dòng 230–294). `docs/design-system/layer3/FLEET-VIEW-SPEC.md` §6 (dòng 93–99). `docs/design-system/layer3/ONBOARDING-SPEC.md` §8 (dòng 158–172). |

---

## 3. Ma Trận Quyết Định Theo Lĩnh Vực Thiết Kế (Decision Matrix)

Bảng đối chiếu tác động thiết kế giữa hai nhánh định hướng đối với từng phân hệ giao diện Layer 3:

| Khu Vực Thiết Kế | Nhánh A: Hộ Gia Đình (Hobbyist Single-Station) | Nhánh B: Thương Mại (Commercial Multi-Station) | Quyết Định Phòng Vệ Layer 3 (Extension Point Đã Dựng) |
|---|---|---|---|
| **Mô Hình Phân Quyền (Roles & Permission Matrix)** | - Chỉ cần 1 tài khoản chủ sở hữu duy nhất (Single-owner login như AeroGarden, Rise Gardens).<br>- Ẩn hoàn toàn trang `/roles` và mục "Quản lý thành viên". Không hiển thị khái niệm Viewer/Operator. | - Phân quyền theo ứng dụng và vị trí địa lý (Priva model: per-site, per-zone).<br>- Bổ sung vai trò chuyên gia (`agronomist`) và kỹ thuật viên bảo trì (`technician`).<br>- Hỗ trợ liên kết tài khoản tư vấn tạm thời có thời hạn tự hủy. | `PermissionMatrix.tsx` chấp nhận mảng động `roles` và `capabilities` (không hardcode 3 cột cố định). Khung mở rộng 4 tầng quyền hạn được ghi nhận trong `PERMISSION-MODEL-SPEC.md` §6. |
| **Form Mời Thành Viên (InviteForm)** | - Người dùng gia đình không mời người qua Firebase UID.<br>- Nếu có chia sẻ, chỉ cần nhập Email và gửi đường dẫn ma thuật (Magic Link) để người thân xem dữ liệu tưới cây. | - Mời theo danh sách nhân viên ca trực; gán quyền chi tiết theo từng trạm/nhà màng cụ thể (Station-scoped role assignment).<br>- Cần ô chọn tổ chức (`organization_id`) và nhóm trạm (`zone_id`). | `InviteForm.tsx` tách biệt 3 khối logic (Identity, Role, Scope Preview), kiểm tra định dạng email và UID độc lập, sẵn sàng mở rộng trường chọn trạm theo dõi. |
| **Giao Diện Quản Lý Cụm (Fleet View Layout)** | - Người dùng chỉ sở hữu 1 trạm; chuyển hướng mặc định từ Dashboard sang bảng điều khiển chi tiết của trạm duy nhất đó.<br>- Fleet View được ẩn hoặc chỉ hiển thị thẻ trạng thái đơn lẻ. | - Quản lý hàng chục đến hàng trăm trạm; bắt buộc phân cụm theo loại cây trồng, khu vực nhà màng (Hick's Law).<br>- Cần chế độ xem bản đồ trực quan (Spatial Greenhouse Map) và nút đẩy công thức dinh dưỡng hàng loạt (Batch Recipe Dispatch). | `FleetStationCard.tsx` và `FleetView.tsx` hỗ trợ tự động gom nhóm khi $\ge 4$ trạm, ưu tiên cảnh báo lên đầu, và chuẩn bị sẵn giao diện chuyển đổi giữa Lưới và Bản đồ. |
| **Quy Trình Khởi Động (Onboarding Flow)** | - Tối giản hóa tối đa: Cắm điện $\to$ Quét mã QR ghép nối $\to$ Nhận dữ liệu cảm biến đầu tiên $\to$ Chọn công thức cây mẫu (Aha Moment đạt được $< 15$ phút). | - Quy trình thiết lập trang trại: Đặt tên tổ chức $\to$ Tạo các khu vực (Zone) $\to$ Quét hàng loạt mã MAC/UID thiết bị $\to$ Phân quyền cho quản lý nông trại và kỹ thuật viên ca trực. | `OnboardingWizard.tsx` và `useOnboardingState.ts` xây dựng theo phễu 4 bước lũy tiến, có sẵn hook lựa chọn persona (`onboarding_persona`) để rẽ nhánh luồng thiết lập. |
| **Chiến Lược Thông Báo (Notification Strategy)** | - Thông báo trực tiếp đẩy về điện thoại chủ nhà khi bình cạn nước hoặc mất điện: "Mực nước bồn thấp, hệ thống an toàn trong 12h" (§5 Minh).<br>- Giảm thiểu tối đa báo động giả để tránh mệt mỏi cảnh báo (Alert fatigue, §12.2). | - Phân luồng thông báo theo ma trận ca trực: Kỹ thuật viên nhận cảnh báo hỏng bơm, quản lý nhận cảnh báo lệch EC/pH, chuyên gia nông học nhận báo cáo tiến độ vụ.<br>- Bắt buộc có cơ chế xác nhận đã xử lý (Alert acknowledgment audit trail). | Backend đã siết chặt quyền xác nhận cảnh báo (`events:write`, SEC-RECIPE-ALERT-SCOPE-001); giao diện cảnh báo sử dụng token chuẩn hóa `Banner` và nhãn danh mục `EVENT_CATEGORY_THEME`. |

---

## 4. Tín Hiệu Tối Thiểu Khả Thi Để Gỡ Chặn (Minimum Viable Signal to Unblock)

Để đưa ra quyết định chuyển dịch chính thức mà không dựa trên suy đoán cảm tính, đội ngũ cần thu thập **1 trong 2** gói tín hiệu thực nghiệm sau:

### 4.1 Tín Hiệu Định Lượng: Bộ Truy Vấn Cơ Sở Dữ Liệu Sản Xuất (§3 SQL Queries)
Chạy trực tiếp 4 câu truy vấn SQL đã soạn thảo trong tài liệu User Discovery (`docs/discovery/2026-09-12-user-discovery.md` §3):

```sql
-- Truy vấn 1: Phân bố số lượng trạm sở hữu trên mỗi người dùng
SELECT 
    device_count,
    COUNT(user_id) AS total_users,
    ROUND((COUNT(user_id)::numeric / SUM(COUNT(user_id)) OVER ()) * 100, 2) AS user_pct
FROM (
    SELECT user_id, COUNT(device_id) AS device_count
    FROM device_ownership
    GROUP BY user_id
) sub
GROUP BY device_count
ORDER BY device_count ASC;

-- Ngưỡng ra quyết định:
-- 1. Nếu người dùng sở hữu >= 2 trạm chiếm > 20%: LẬP TỨC MỞ KHÓA NHÁNH B (Thương mại / Fleet View ưu tiên).
-- 2. Nếu người dùng sở hữu = 1 trạm chiếm >= 85%: XÁC NHẬN NHÁNH A (Hộ gia đình là trọng tâm, ẩn bớt tính năng đa trạm).
```

### 4.2 Tín Hiệu Định Tính: Bộ Phỏng Vấn Tối Thiểu (3 trên 5 Nhóm Hình Mẫu)
Thực hiện tối thiểu 3 cuộc phỏng vấn sâu dài 45 phút theo kịch bản mẫu tại `docs/discovery/2026-09-12-user-discovery.md` §7, bao gồm:
1. **01 Người trồng ban công đô thị (Minh Archetype):** Xác định sự sẵn sàng chi trả cho ứng dụng di động so với mong muốn sở hữu phần cứng đóng gói sẵn.
2. **01 Quản lý trang trại thương mại vừa/nhỏ (Anh Thắng Archetype):** Xác minh quy trình phân công ca trực hàng ngày và yêu cầu phân quyền thao tác bơm/thuốc.
3. **01 Kỹ sư công nghệ sinh học / Nhà nghiên cứu nông nghiệp:** Đánh giá tính cần thiết của việc trích xuất dữ liệu thô và khả năng quản lý nhiều tháp khí canh cùng lúc.

---

## 5. Danh Mục Các Điểm Mở Rộng Đã Tạo Trong Layer 3 (Extension Points Catalog)

Trong đợt triển khai Layer 3, các thành viên Track A–D đã cài đặt sẵn các "chốt mở rộng" phòng vệ mã nguồn:

| Nhóm / Thành Viên | Vị Trí File Mã Nguồn | Tên Điểm Mở Rộng | Mô Tả Kỹ Thuật & Cách Thức Mở Rộng |
|---|---|---|---|
| **Track A (Roles Component)** | `hydragrow-frontend/src/components/roles/PermissionMatrix.tsx` | Dynamic Roles & Capabilities Array Props | Không khóa cứng 3 cột `admin`/`operator`/`viewer`. Chấp nhận mảng `roles: UserRole[]` và `capabilities: Capability[]`. Sẵn sàng nạp vai trò `technician`, `agronomist` và quyền theo trạm (`action:resource:id`). |
| **Track B (Roles Form)** | `hydragrow-frontend/src/components/roles/InviteForm.tsx` | Chunked Form Architecture & Scope Hooks | Kiến trúc form tách 3 nhóm độc lập (Identity, Role Selection, Summary Preview). Validation Email và Firebase UID phân lập. Dễ dàng bổ sung dropdown chọn Trạm/Khu vực áp dụng quyền mà không phá vỡ layout. |
| **Track C (Fleet UX)** | `hydragrow-frontend/src/components/fleet/FleetStationCard.tsx`, `FleetView.tsx` | Dynamic Grouping & Warning Sort | Tự động chuyển đổi giữa lưới đơn giản ($< 4$ trạm) và gom nhóm theo mùa vụ ($\ge 4$ trạm). Sắp xếp ưu tiên trạm có cảnh báo. Đã trừ sẵn không gian cho nút chuyển đổi sang Bản đồ nhà màng (Spatial Layout). |
| **Track D (Onboarding)** | `hydragrow-frontend/src/components/onboarding/OnboardingWizard.tsx`, `useOnboardingState.ts` | Persona-specific Wizard Extension Hook | Trạng thái lưu trong `localStorage` với cờ mở rộng `onboarding_persona`. Cho phép chèn bước chọn "Trang trại quy mô lớn" để kích hoạt luồng kết nối hàng loạt (Batch Pairing) thay vì quét từng mã. |

---

## 6. Đề Xuất Kiến Trúc Sản Phẩm Đa Tầng (Tiered Product Architecture Proposal)

*Lưu ý: Đây là đề xuất kiến trúc thiết kế để giải quyết xung đột, không phải quyết định kinh doanh mang tính áp đặt.*

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    HYDRAGROW SHARED CORE PLATFORM                       │
│  - PostgreSQL (`device_ownership`, `dosing_reports`, `crop_seasons`)    │
│  - InfluxDB Time-Series Engine (`bucket: sensors`)                      │
│  - ESP32 Controller Hardware + MQTT Broker Broker + FSM Safety Interlock│
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                 ┌───────────────────┴───────────────────┐
                 ▼                                       ▼
┌──────────────────────────────────┐   ┌──────────────────────────────────┐
│         FREE / HOBBY TIER        │   │       COMMERCIAL PRO TIER        │
│      "Trải Nghiệm An Tâm"        │   │     "Tối Ưu Năng Suất Quy Mô"    │
├──────────────────────────────────┤   ├──────────────────────────────────┤
│ • Người dùng: Tối đa 1 tài khoản │   │ • Người dùng: Không giới hạn     │
│ • Thiết bị: Giới hạn 1 trạm tháp │   │ • Thiết bị: Cụm đa trạm / Vùng   │
│ • Giao diện: Dashboard trực quan,│   │ • Giao diện: Fleet View phân cụm,│
│   không có trang Roles/Phân quyền│   │   Ma trận quyền hạn đa vai trò   │
│ • Cảnh báo: Đẩy thông báo cơ bản │   │ • Cảnh báo: Ma trận ca trực, ghi │
│   (cạn nước, mất kết nối Wi-Fi)  │   │   nhật ký kiểm toán (Audit Trail)│
│ • Công thức: Chọn mẫu có sẵn     │   │ • Công thức: Tùy biến sâu, batch │
│   (Xà lách, Dâu tây, Rau muống)  │   │   dispatch hàng loạt ra nhiều trạm│
└──────────────────────────────────┘   └──────────────────────────────────┘
```

---

## 7. Đánh Giá Rủi Ro Khi Q1 Tiếp Tục Bị Chặn (Risk Assessment)

Nếu câu hỏi Q1 tiếp tục bị thả nổi trong 3 tháng tới mà không có quyết định dứt khoát hoặc dữ liệu thực nghiệm gỡ chặn:

1. **Tích tụ nợ kỹ thuật và nợ thiết kế (Design Debt Accumulation):**
   - Đội ngũ Frontend sẽ tiếp tục phải viết code "nửa nạc nửa mỡ" (vừa hỗ trợ đa trạm ngầm, vừa tối ưu cho 1 trạm).
   - Các điểm mở rộng phòng vệ (Extension Points) nếu không được kích hoạt sẽ trở thành mã nguồn thừa (dead code), làm tăng chi phí kiểm thử tự động và bảo trì hồi quy.
2. **Nguy cơ cạnh tranh từ thị trường (§11.1 – §11.3):**
   - Nếu tập trung vào hộ gia đình nhưng thiếu phần cứng đóng gói gọn gàng, người dùng sẽ chọn các giải pháp trọn gói như Rise Gardens hoặc Click & Grow.
   - Nếu để thị trường thương mại trống trải, các đối thủ như Growlink ($50–$1,000/tháng) hoặc Autogrow sẽ chiếm lĩnh phân khúc trang trại vừa và nhỏ tại Đông Nam Á bằng tính năng phân quyền thương mại chuyên nghiệp.
3. **Mệt mỏi nhận thức của nhà phát triển (Developer Cognitive Fatigue):**
   - Mọi thảo luận về tính năng mới (như Lịch sử tưới, Bản đồ cảm biến, Báo cáo dinh dưỡng) đều sẽ bị đình trệ vì liên tục vấp phải câu hỏi: *"Tính năng này làm cho ai? Minh hay Anh Thắng?"*.

---

## 8. Quy Tắc Quản Trị Bắt Buộc (Governance Rule)

> **QUY TẮC BẮT BUỘC:**  
> **Không có bất kỳ tính năng phụ thuộc Q1 nào được phép triển khai vào sản phẩm thương mại mà không cập nhật tài liệu `Q1-DECISION-FRAMEWORK.md` này để ghi nhận rõ ràng nhánh quyết định đã được lựa chọn, đi kèm bằng chứng thực nghiệm đối chiếu.**
