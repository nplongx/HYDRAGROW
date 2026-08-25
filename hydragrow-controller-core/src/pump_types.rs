#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PumpType {
    NutrientA,
    NutrientB,
    PhUp,
    PhDown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaterDirection {
    In,
    Out,
    Stop,
}
