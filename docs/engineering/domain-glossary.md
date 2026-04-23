# Domain Glossary & Naming Conventions

## Mục tiêu
Tài liệu này định nghĩa **tên chuẩn cho các thuật ngữ domain** và **quy tắc đặt tên kỹ thuật** để đảm bảo toàn bộ codebase dùng cùng một ngôn ngữ.

> Bắt buộc: mọi thay đổi refactor naming phải đối chiếu glossary này trước khi merge.

---

## 1) Thuật ngữ domain chuẩn
Mỗi thuật ngữ domain chỉ có **một tên chuẩn duy nhất** để sử dụng trong code, API contract, schema và tài liệu.

| Tên chuẩn | Mô tả ngắn |
|---|---|
| `customer` | Khách hàng sử dụng dịch vụ/sản phẩm |
| `order` | Đơn hàng/phiên giao dịch mua |
| `product` | Sản phẩm |
| `payment` | Thanh toán |
| `invoice` | Hóa đơn |
| `subscription` | Gói đăng ký |
| `shipment` | Vận chuyển/giao hàng |
| `address` | Địa chỉ |
| `user` | Tài khoản đăng nhập/hệ thống |
| `role` | Vai trò phân quyền |

> Lưu ý: Khi có thuật ngữ mới, phải bổ sung vào bảng này trước khi dùng rộng rãi.

---

## 2) Alias cũ -> tên chuẩn
Tất cả alias cũ cần được chuẩn hóa về tên chuẩn tương ứng.

| Alias cũ | Tên chuẩn |
|---|---|
| `cust` | `customer` |
| `customerInfo` | `customer` |
| `client` | `customer` |
| `usr` | `user` |
| `accountUser` | `user` |
| `purchase` | `order` |
| `txn`, `transaction` | `payment` *(nếu ngữ cảnh là thanh toán)* |
| `bill` | `invoice` |
| `sub` | `subscription` |
| `delivery` | `shipment` |

### Quy tắc chuẩn hóa alias
- Tên biến, tên field, tên method, tên file, tên module phải dùng **tên chuẩn**.
- Không introduce alias mới nếu chưa được phê duyệt và cập nhật vào glossary.
- Trường hợp mơ hồ ngữ nghĩa (`transaction` có thể không phải `payment`), team phải thống nhất domain context trước khi rename.

---

## 3) Quy tắc tiền tố/hậu tố kỹ thuật
Mọi naming theo cấu trúc: `<DomainTerm><TypeSuffix>` (PascalCase) cho class/type.

### DTO
- Hậu tố bắt buộc: `Dto`.
- Mẫu: `<Domain><Action?>Dto`.
- Ví dụ: `CustomerDto`, `CreateCustomerDto`, `UpdateCustomerDto`.

### Entity
- Hậu tố bắt buộc: `Entity`.
- Mẫu: `<Domain>Entity`.
- Ví dụ: `CustomerEntity`, `OrderEntity`.

### Service
- Hậu tố bắt buộc: `Service`.
- Mẫu: `<Domain>Service`.
- Ví dụ: `CustomerService`, `PaymentService`.

### Repository
- Hậu tố bắt buộc: `Repository`.
- Mẫu: `<Domain>Repository`.
- Ví dụ: `CustomerRepository`, `InvoiceRepository`.

### Handler
- Hậu tố bắt buộc: `Handler`.
- Mẫu: `<Action><Domain>Handler` hoặc `<Domain><Action>Handler` (chọn 1 style và thống nhất theo module).
- Ví dụ: `CreateCustomerHandler`, `CustomerCreatedHandler`.

### Quy ước tiền tố/hậu tố bổ sung
- Interface (nếu dùng): tiền tố `I` + tên chuẩn, ví dụ: `ICustomerService`.
- Biến instance service/repository: camelCase theo tên chuẩn, ví dụ: `customerService`, `orderRepository`.
- File name nên phản ánh class/type name (kebab-case hoặc PascalCase theo convention repo), nhưng vẫn phải giữ tên domain chuẩn.

---

## 4) Quy định merge cho refactor naming
Trước khi merge bất kỳ PR có thay đổi naming:

1. **Đối chiếu glossary**: mọi tên domain phải map về tên chuẩn.
2. **Kiểm tra alias**: không còn alias cũ trong code mới/chỗ đã refactor.
3. **Kiểm tra hậu tố/tiền tố**: DTO/Entity/Service/Repository/Handler tuân thủ quy tắc ở mục 3.
4. **Cập nhật tài liệu liên quan**: nếu phát sinh thuật ngữ mới, phải cập nhật file này trong cùng PR.

> Chính sách bắt buộc: **Mọi refactor naming phải bám theo glossary này trước khi merge.**
