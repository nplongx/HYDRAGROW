#[derive(Debug, Clone, Default)]
pub struct DoseVector {
    pub nutrient_a_ml: f32,
    pub nutrient_b_ml: f32,
    pub ph_up_ml: f32,
}

#[derive(Debug, Clone, Default)]
pub struct ResponseVector {
    pub ec_delta: f32,
    pub ph_delta: f32,
}

#[derive(Debug, Clone)]
pub struct InteractionMatrix {
    pub data: [[f32; 3]; 2],
}

impl Default for InteractionMatrix {
    fn default() -> Self {
        Self {
            data: [[0.0; 3]; 2],
        }
    }
}

impl InteractionMatrix {
    pub fn from_scalar(ec_gain_per_ml: f32, ph_shift_up_per_ml: f32) -> Self {
        Self {
            data: [
                [ec_gain_per_ml, ec_gain_per_ml, 0.0],
                [0.0, 0.0, ph_shift_up_per_ml],
            ],
        }
    }

    pub fn predict(&self, dose: &DoseVector) -> ResponseVector {
        let doses = [dose.nutrient_a_ml, dose.nutrient_b_ml, dose.ph_up_ml];
        let ec_delta =
            self.data[0][0] * doses[0] + self.data[0][1] * doses[1] + self.data[0][2] * doses[2];
        let ph_delta =
            self.data[1][0] * doses[0] + self.data[1][1] * doses[1] + self.data[1][2] * doses[2];

        ResponseVector { ec_delta, ph_delta }
    }

    pub fn update_column(
        &mut self,
        col: usize,
        dose_ml: f32,
        observed: f32,
        row: usize,
        gain_k: f32,
    ) {
        if row >= 2 || col >= 3 || dose_ml.abs() < 1e-6 {
            return;
        }

        let predicted = self.data[row][col] * dose_ml;
        let residual = observed - predicted;
        let step = gain_k.clamp(0.0, 1.0) * residual / dose_ml;
        self.data[row][col] = (self.data[row][col] + step).clamp(-10_000.0, 10_000.0);
    }

    pub fn get(&self, row: usize, col: usize) -> f32 {
        if row < 2 && col < 3 {
            self.data[row][col]
        } else {
            0.0
        }
    }

    pub fn as_flat(&self) -> [f32; 6] {
        [
            self.data[0][0],
            self.data[0][1],
            self.data[0][2],
            self.data[1][0],
            self.data[1][1],
            self.data[1][2],
        ]
    }

    pub fn from_flat(flat: [f32; 6]) -> Self {
        Self {
            data: [[flat[0], flat[1], flat[2]], [flat[3], flat[4], flat[5]]],
        }
    }
}

#[derive(Debug, Clone)]
pub struct KalmanCovarianceDiag {
    pub p: [f32; 3],
    pub q: f32,
    pub r: f32,
}

impl KalmanCovarianceDiag {
    pub fn new(p0: f32, q: f32, r: f32) -> Self {
        let p0 = p0.max(0.0);
        Self {
            p: [p0; 3],
            q: q.max(0.0),
            r: r.max(1e-9),
        }
    }

    pub fn predict(&mut self) {
        for p_i in &mut self.p {
            *p_i += self.q;
        }
    }

    pub fn update_and_get_gain(&mut self, idx: usize) -> f32 {
        if idx >= self.p.len() {
            return 0.0;
        }

        let p = self.p[idx].max(0.0);
        let denom = p + self.r;
        if denom <= 1e-9 {
            return 0.0;
        }

        let k = (p / denom).clamp(0.0, 1.0);
        self.p[idx] = ((1.0 - k) * p).max(1e-9);
        k
    }

    pub fn update(&mut self, idx: usize) {
        let _ = self.update_and_get_gain(idx);
    }

    pub fn confidence(&self, idx: usize) -> f32 {
        if idx >= self.p.len() {
            return 0.0;
        }
        1.0 / (1.0 + self.p[idx].max(0.0))
    }
}
