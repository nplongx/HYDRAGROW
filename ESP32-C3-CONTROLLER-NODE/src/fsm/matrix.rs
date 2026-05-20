#[derive(Debug, Clone, Copy, Default)]
pub struct EcMatrix {
    pub g_ec_a: f32,
    pub step_ratio_ec: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PhMatrix {
    pub g_ph_x: f32,
    pub step_ratio_ph: f32,
    pub ph_tolerance: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CouplingMatrix {
    pub ph_per_ml_a: f32,
    pub ph_per_ml_b: f32,
}
