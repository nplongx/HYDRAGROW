-- hydragrow-backend/migrations/20260910100002_add_light_hours_to_crop_recipe_stages.sql
-- Thêm số giờ chiếu sáng/ngày theo giai đoạn sinh trưởng (ghi chép hiển thị RecipeBuilder).
ALTER TABLE crop_recipe_stages
ADD COLUMN IF NOT EXISTS light_hours INT;