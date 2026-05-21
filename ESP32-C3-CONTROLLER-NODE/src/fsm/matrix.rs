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
    /// Chuyển đổi vector cấu trúc sang mảng phẳng 8 phần tử để phục vụ phép toán ma trận
    pub fn as_array(&self) -> [f32; 8] {
        [
            self.nutrient_a_ml,
            self.nutrient_b_ml,
            self.ph_up_ml,
            self.ph_down_ml,
            self.water_in_sec,
            self.water_out_sec,
            self.mixing_sec,
            self.misting_sec,
        ]
    }

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

    /// Phép nhân ma trận: StateDeltaVector (4x1) = InteractionMatrix (4x8) * ControlVector (8x1)
    pub fn predict(&self, control: &ControlVector) -> StateDeltaVector {
        let u = control.as_array();
        let mut y = [0.0; 4];

        for row in 0..4 {
            let mut sum = 0.0;
            for col in 0..8 {
                sum += self.data[row][col] * u[col];
            }
            y[row] = sum;
        }

        StateDeltaVector {
            ec_delta: y[0],
            ph_delta: y[1],
            water_level_delta: y[2],
            temp_delta: y[3],
        }
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

    pub fn get(&self, row: usize, col: usize) -> f32 {
        if row < 4 && col < 8 {
            self.data[row][col]
        } else {
            0.0
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
        for r in 0..4 {
            for c in 0..8 {
                t[c][r] = self.data[r][c];
            }
        }
        t
    }

    /// Nhân ma trận 4x8 với ma trận chuyển vị 8x4 để tạo ra ma trận vuông 4x4 (M * M^T)
    fn multiply_by_transpose(&self, t: &[[f32; 4]; 8]) -> [[f32; 4]; 4] {
        let mut res = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                let mut sum = 0.0;
                for k in 0..8 {
                    sum += self.data[i][k] * t[k][j];
                }
                res[i][j] = sum;
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
            for j in 0..8 {
                aug[i][j] /= pivot;
            }

            // Khử các hàng còn lại
            for k in 0..4 {
                if k != i {
                    let factor = aug[k][i];
                    for j in 0..8 {
                        aug[k][j] -= factor * aug[i][j];
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

#[derive(Debug, Clone)]
pub struct KalmanCovarianceDiag {
    /// Mở rộng p lên mảng 8 trục độc lập ước lượng lỗi cho 8 cơ cấu chấp hành phần cứng
    pub p: [f32; 8],
    pub q: f32,
    pub r: f32,
}

impl KalmanCovarianceDiag {
    pub fn new(p0: f32, q: f32, r: f32) -> Self {
        let p0 = p0.max(0.0);
        Self {
            p: [p0; 8],
            q: q.max(0.0),
            r: r.max(1e-9),
        }
    }

    /// Dự đoán bước tiến Kalman (Cộng thêm nhiễu hệ thống Q theo thời gian để giảm độ tự tin)
    pub fn predict(&mut self) {
        for p_i in &mut self.p {
            *p_i += self.q;
        }
    }

    /// Cập nhật trạng thái bộ lọc và tính toán ma trận tăng Kalman Gain K phục vụ thuật toán học RLS
    pub fn update_and_get_gain(&mut self, idx: usize) -> f32 {
        if idx >= self.p.len() {
            return 0.0;
        }

        let p_val = self.p[idx].max(0.0);
        let denom = p_val + self.r; // r là nhiễu đo lường của cảm biến
        if denom <= 1e-9 {
            return 0.0;
        }

        let k = (p_val / denom).clamp(0.0, 1.0);
        // Ép phương sai lỗi nhỏ xuống thể hiện độ hội tụ toán học tăng cao
        self.p[idx] = ((1.0 - k) * p_val).max(1e-9);
        k
    }

    /// Hàm tính toán toán học chuẩn xác độ tự tin (Confidence) biến thiên mịn [0.0 -> 1.0]
    pub fn confidence(&self, idx: usize) -> f32 {
        if idx >= self.p.len() {
            return 0.0;
        }
        1.0 / (1.0 + self.p[idx].max(0.0))
    }
}
