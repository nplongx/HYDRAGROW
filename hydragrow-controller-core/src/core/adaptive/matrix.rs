use crate::core::adaptive::kalman::KalmanCovarianceDiag;
use crate::core::fsm::types::PendingCalibrationSample;

#[derive(Debug, Clone, Default)]
pub struct ControlVector {
    pub nutrient_a_ml: f32,
    pub nutrient_b_ml: f32,
    pub ph_up_ml: f32,
    pub ph_down_ml: f32,
    pub water_in_sec: f32,
    pub water_out_sec: f32,
    pub mixing_sec: f32,
    pub misting_sec: f32,
}

impl ControlVector {
    /// Khôi phục cấu trúc vector từ một mảng phẳng 8 phần tử
    pub fn from_array(arr: [f32; 8]) -> Self {
        Self {
            nutrient_a_ml: arr[0],
            nutrient_b_ml: arr[1],
            ph_up_ml: arr[2],
            ph_down_ml: arr[3],
            water_in_sec: arr[4],
            water_out_sec: arr[5],
            mixing_sec: arr[6],
            misting_sec: arr[7],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StateDeltaVector {
    pub ec_delta: f32,
    pub ph_delta: f32,
    pub water_level_delta: f32,
    pub temp_delta: f32,
}

#[derive(Debug, Clone)]
pub struct InteractionMatrix {
    /// Ma trận tương tác kích thước 4 hàng (chỉ số) x 8 cột (thiết bị điều khiển)
    pub data: [[f32; 8]; 4],
}

impl Default for InteractionMatrix {
    fn default() -> Self {
        Self {
            data: [[0.0; 8]; 4],
        }
    }
}

impl InteractionMatrix {
    /// Khởi tạo ma trận từ các hằng số cấu hình tĩnh ban đầu
    pub fn from_scalar(
        ec_gain_per_ml: f32,
        ph_shift_up_per_ml: f32,
        ph_shift_down_per_ml: f32,
        water_in_cm_per_sec: f32,
        water_out_cm_per_sec: f32,
    ) -> Self {
        let mut m = Self::default();

        // --- HÀNG 0: Biến thiên EC ---
        m.data[0][0] = ec_gain_per_ml; // Dinh dưỡng A làm tăng EC (+)
        m.data[0][1] = ec_gain_per_ml; // Dinh dưỡng B làm tăng EC (+)
        m.data[0][4] = -0.005; // Bơm nước vào mặc định pha loãng làm giảm EC (-)

        // --- HÀNG 1: Biến thiên pH ---
        m.data[1][2] = ph_shift_up_per_ml; // Thuốc pH Up làm tăng pH (+)
        m.data[1][3] = -ph_shift_down_per_ml.abs(); // Thuốc pH Down làm giảm pH (-)
        m.data[1][4] = 0.002;

        // --- HÀNG 2: Biến thiên Mức nước (Water Level) ---
        m.data[2][4] = water_in_cm_per_sec; // Bơm nước vào làm tăng mực nước (+)
        m.data[2][5] = -water_out_cm_per_sec.abs(); // Bơm xả nước làm giảm mực nước (-)
        m.data[2][7] = -0.001; // Phun sương hao nước từ bồn (-)

        // --- HÀNG 3: Biến thiên Nhiệt độ (Temperature) ---
        m.data[3][7] = -0.02; // Phun sương bay hơi làm giảm nhiệt độ nước/môi trường (-)

        m
    }

    pub fn update_column(
        &mut self,
        col: usize,
        input_value: f32,
        observed_delta: f32,
        row: usize,
        gain_k: f32,
    ) {
        if row >= 4 || col >= 8 || input_value.abs() < 1e-6 {
            return;
        }

        let predicted_delta = self.data[row][col] * input_value;
        let residual = observed_delta - predicted_delta;

        // Cập nhật trọng số của ô ma trận dựa trên hệ số tăng Kalman K
        let step = gain_k.clamp(0.0, 1.0) * residual / input_value;
        self.data[row][col] = (self.data[row][col] + step).clamp(-10_000.0, 10_000.0);
    }

    pub fn update_matrix_adaptive(
        &mut self,
        kalman: &mut KalmanCovarianceDiag,
        sample: &PendingCalibrationSample,
        post_ec: f32,
        post_ph: f32,
        post_water: f32,
        post_temp: f32,
    ) {
        kalman.predict();

        let delta_ec = post_ec - sample.start_ec;
        let delta_ph = post_ph - sample.start_ph;
        let delta_water = post_water - sample.start_water_level;
        let delta_temp = post_temp - sample.start_temp;

        // --- CỘT 0 & 1: Học đặc tính châm thuốc phân bón (Hàng 0: EC) ---
        if sample.dose_a_ml > 0.0 {
            let k0 = kalman.update_and_get_gain(0);
            self.update_column(0, sample.dose_a_ml, delta_ec, 0, k0);
        }
        if sample.dose_b_ml > 0.0 {
            let k1 = kalman.update_and_get_gain(1);
            self.update_column(1, sample.dose_b_ml, delta_ec, 0, k1);
        }

        // --- CỘT 2 & 3: Học đặc tính axit/kiềm (Hàng 1: pH) ---
        if sample.dose_ph_up_ml > 1e-3 {
            let k2 = kalman.update_and_get_gain(2);
            self.update_column(2, sample.dose_ph_up_ml, delta_ph, 1, k2);
        }
        if sample.dose_ph_down_ml > 1e-3 {
            let k3 = kalman.update_and_get_gain(3);
            self.update_column(3, sample.dose_ph_down_ml, delta_ph, 1, k3);
        }

        // --- CỘT 4: Học đặc tính Bơm cấp nước vào (Hàng 2: Mực nước, Hàng 0: EC & Hàng 1: pH) ---
        if sample.water_in_sec > 0.1 {
            let k4 = kalman.update_and_get_gain(4);
            self.update_column(4, sample.water_in_sec, delta_water, 2, k4);
            self.update_column(4, sample.water_in_sec, delta_ec, 0, k4);
            self.update_column(4, sample.water_in_sec, delta_ph, 1, k4);
        }

        // --- CỘT 5: Học đặc tính Bơm xả nước ra ngoài (Hàng 2: Mực nước) ---
        if sample.water_out_sec > 0.1 {
            let k5 = kalman.update_and_get_gain(5);
            self.update_column(5, sample.water_out_sec, delta_water, 2, k5);
        }

        // --- CỘT 6: Bơm trộn tuần hoàn Osaka ---
        let actual_osaka_sec = (sample
            .active_mixing_finish_ms
            .saturating_sub(sample.start_ms) as f32
            / 1000.0)
            .clamp(0.0, 300.0);
        if actual_osaka_sec > 1.0 {
            let k6 = kalman.update_and_get_gain(6);
            self.update_column(6, actual_osaka_sec, delta_water, 2, k6);
            self.update_column(6, actual_osaka_sec, delta_ec * 0.1, 0, k6 * 0.1);
            self.update_column(6, actual_osaka_sec, delta_ph * 0.1, 1, k6 * 0.1);
        }

        // --- CỘT 7: Van phun sương giải nhiệt ---
        let actual_misting_sec = (sample
            .stabilizing_finish_ms
            .unwrap_or(sample.start_ms)
            .saturating_sub(sample.start_ms) as f32
            / 1000.0)
            .min(30.0);
        if actual_misting_sec > 0.1 {
            let k7 = kalman.update_and_get_gain(7);
            self.update_column(7, actual_misting_sec, delta_water, 2, k7);
            self.update_column(7, actual_misting_sec, delta_temp, 3, k7);
            self.update_column(7, actual_misting_sec, delta_ec * 0.1, 0, k7 * 0.1);
            self.update_column(7, actual_misting_sec, delta_ph * 0.1, 1, k7 * 0.1);
        }
    }

    /// Làm phẳng ma trận 4x8 thành mảng 32 phần tử tuần tự hàng để lưu xuống Flash (NVS Snapshot)
    pub fn as_flat(&self) -> [f32; 32] {
        let mut flat = [0.0; 32];
        let mut idx = 0;
        for row in 0..4 {
            for col in 0..8 {
                flat[idx] = self.data[row][col];
                idx += 1;
            }
        }
        flat
    }

    /// Khôi phục cấu trúc ma trận 4x8 từ mảng phẳng 32 phần tử đọc lên từ Flash NVS
    pub fn from_flat(flat: [f32; 32]) -> Self {
        let mut m = Self::default();
        let mut idx = 0;
        for row in 0..4 {
            for col in 0..8 {
                m.data[row][col] = flat[idx];
                idx += 1;
            }
        }
        m
    }

    // --- CÁC PHÉP TOÁN ĐẠI SỐ TUYẾN TÍNH MẢNG TĨNH PHỤ TRỢ ---

    /// Tính chuyển vị ma trận (8x4 từ 4x8)
    fn transpose(&self) -> [[f32; 4]; 8] {
        let mut t = [[0.0; 4]; 8];
        for (r, row) in self.data.iter().enumerate().take(4) {
            for (c, val) in row.iter().enumerate().take(8) {
                t[c][r] = *val;
            }
        }
        t
    }

    /// Nhân ma trận 4x8 với ma trận chuyển vị 8x4 để tạo ra ma trận vuông 4x4 (M * M^T)
    fn multiply_by_transpose(&self, t: &[[f32; 4]; 8]) -> [[f32; 4]; 4] {
        let mut res = [[0.0; 4]; 4];
        for (i, row_res) in res.iter_mut().enumerate() {
            for (j, item_res) in row_res.iter_mut().enumerate() {
                let mut sum = 0.0;
                for (k, val_t) in t.iter().enumerate().take(8) {
                    sum += self.data[i][k] * val_t[j];
                }
                *item_res = sum;
            }
        }
        res
    }

    /// Nghịch đảo ma trận vuông 4x4 bằng phương pháp loại trừ Gauss-Jordan
    /// Trả về None nếu ma trận bị suy biến (Suy biến tức là không có thiết bị phần cứng nào hoạt động để tạo ra thay đổi)
    fn invert_4x4(&self, m: [[f32; 4]; 4]) -> Option<[[f32; 4]; 4]> {
        let mut aug = [[0.0; 8]; 4];
        // Khởi tạo ma trận bổ sung [M | I]
        for i in 0..4 {
            for j in 0..4 {
                aug[i][j] = m[i][j];
            }
            aug[i][i + 4] = 1.0;
        }

        // Khử toàn bộ Gauss-Jordan
        for i in 0..4 {
            // Tìm phần tử chốt lớn nhất để tăng tính ổn định số học (Pivot)
            let mut max_row = i;
            for k in (i + 1)..4 {
                if aug[k][i].abs() > aug[max_row][i].abs() {
                    max_row = k;
                }
            }
            if max_row != i {
                aug.swap(i, max_row);
            }

            let pivot = aug[i][i];
            if pivot.abs() < 1e-6 {
                return None; // Ma trận suy biến, hệ thống bị mất kiểm soát cục bộ
            }

            // Chia hàng hiện tại cho phần tử chốt để đưa phần tử đường chéo chính về 1
            for item in &mut aug[i] {
                *item /= pivot;
            }

            // Khử các hàng còn lại
            for k in 0..4 {
                if k != i {
                    let factor = aug[k][i];
                    let aug_i = aug[i];
                    for (aug_k_j, &aug_i_j) in aug[k].iter_mut().zip(aug_i.iter()) {
                        *aug_k_j -= factor * aug_i_j;
                    }
                }
            }
        }

        // Tách ma trận nghịch đảo ra từ nửa phải của ma trận bổ sung
        let mut inv = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                inv[i][j] = aug[i][j + 4];
            }
        }
        Some(inv)
    }

    /// Nhân ma trận chuyển vị (8x4) với ma trận nghịch đảo (4x4) để tạo ma trận Giả nghịch đảo Moore-Penrose (8x4)
    fn multiply_transpose_by_inverse(
        &self,
        t: &[[f32; 4]; 8],
        inv: &[[f32; 4]; 4],
    ) -> [[f32; 4]; 8] {
        let mut pseudo = [[0.0; 4]; 8];
        for i in 0..8 {
            for j in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += t[i][k] * inv[k][j];
                }
                pseudo[i][j] = sum;
            }
        }
        pseudo
    }

    // --- BỘ GIẢI TOÁN TOÀN DIỆN (MPC SOLVER) ---

    /// Giải hệ phương trình MIMO: Tìm cấu hình ControlVector tối ưu từ sai số StateDeltaVector
    pub fn solve(&self, target_delta: &StateDeltaVector) -> Option<ControlVector> {
        // 1. Tính chuyển vị M^T
        let m_t = self.transpose();

        // 2. Tính ma trận vuông (M * M^T)
        let m_m_t = self.multiply_by_transpose(&m_t);

        // 3. Tính nghịch đảo (M * M^T)^-1
        let inv_m_m_t = self.invert_4x4(m_m_t)?;

        // 4. Tính ma trận Giả nghịch đảo Moore-Penrose: M^+ = M^T * (M * M^T)^-1
        let pseudo_inverse = self.multiply_transpose_by_inverse(&m_t, &inv_m_m_t);

        // 5. Nhân Giả nghịch đảo với Vector sai số đầu vào để tìm kết quả: u = M^+ * y
        let y = [
            target_delta.ec_delta,
            target_delta.ph_delta,
            target_delta.water_level_delta,
            target_delta.temp_delta,
        ];
        let mut u = [0.0; 8];

        for i in 0..8 {
            let mut sum = 0.0;
            for j in 0..4 {
                sum += pseudo_inverse[i][j] * y[j];
            }
            u[i] = sum;
        }

        // 6. TẦNG RÀNG BUỘC VẬT LÝ VÀ AN TOÀN (Constraints Guardrails)
        // Hệ thống nhúng không thể thực hiện các hành động "âm vật lý" (không thể hút ngược thuốc).
        // Chúng ta lọc nhiễu số học và giữ lại các xung lực dương thực thi hợp lệ.
        let mut control_arr = [0.0; 8];
        for i in 0..8 {
            if u[i] > 1e-4 {
                control_arr[i] = u[i];
            } else {
                control_arr[i] = 0.0; // Ép về 0 đối với các kết quả âm vật lý hoặc sai số tính toán siêu nhỏ
            }
        }

        Some(ControlVector::from_array(control_arr))
    }
}
