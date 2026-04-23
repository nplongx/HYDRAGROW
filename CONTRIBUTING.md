# Contributing Guide

## 1) Naming conventions

### General

- Tên phải rõ nghĩa theo domain HYDRAGROW, tránh viết tắt mơ hồ.
- Một khái niệm chỉ dùng **một** tên thống nhất theo glossary bên dưới.
- Tránh typo và biến thể không nhất quán (ví dụ: `moisture` vs `moistur`).

### Code style theo loại tên

- **Type / Struct / Enum / Component**: `PascalCase`
- **Function / method / variable**: `camelCase` (TS/JS) hoặc `snake_case` (Rust)
- **Constant**: `UPPER_SNAKE_CASE`
- **File TypeScript React component**: `PascalCase.tsx`
- **File utility/hook/service**: `camelCase.ts` (frontend), `snake_case.rs` (Rust)

## 2) Glossary (bắt buộc tuân thủ)

Dùng đúng các thuật ngữ chuẩn sau trong code, docs, commit và PR:

- `HydraGrow` (brand/product name)
- `sensor node`
- `controller node`
- `dosing`
- `nutrient tank`
- `system event`
- `schedule` / `scheduled dosing`

## 3) Ví dụ đúng / sai cho lỗi naming thường gặp

### Ví dụ 1: Không thống nhất brand name

- ✅ Đúng: `HydraGrowDashboard`
- ❌ Sai: `HydragrowDashboard`, `HYDRAGROWDashboard`

### Ví dụ 2: Sai chuẩn casing theo ngôn ngữ

- ✅ Đúng (Rust fn): `calculate_dosing_volume`
- ❌ Sai: `calculateDosingVolume`

- ✅ Đúng (TS fn): `calculateDosingVolume`
- ❌ Sai: `calculate_dosing_volume`

### Ví dụ 3: Sai chính tả domain term

- ✅ Đúng: `moistureThreshold`
- ❌ Sai: `moisturThreshold`, `moistureTreshold`

### Ví dụ 4: Tên file component React

- ✅ Đúng: `NutrientTankCard.tsx`
- ❌ Sai: `nutrient-tank-card.tsx`, `nutrientTankCard.tsx`

### Ví dụ 5: Viết tắt mơ hồ

- ✅ Đúng: `scheduledDosing`
- ❌ Sai: `schDose`, `sd`

## 4) Pull request checklist

Trước khi submit PR, xác nhận:

- [ ] Đã chạy format/lint/check tương ứng với module thay đổi.
- [ ] Đã tuân thủ naming conventions trong tài liệu này.
- [ ] Đã dùng đúng glossary terms chuẩn.
- [ ] Không đưa secrets/keys/config nhạy cảm vào commit.
- [ ] Mô tả rõ phạm vi thay đổi và cách kiểm thử.
