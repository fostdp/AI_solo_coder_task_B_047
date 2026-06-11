pub mod algorithm {
    pub const SYNTAX_CHUNK_SIZE: usize = 32;
    pub const SYNTAX_NODE_THRESHOLD_CHUNKED: usize = 500;
    pub const SYNTAX_NODE_THRESHOLD_OPTIMIZED_GRAPH: usize = 100;
    pub const SYNTAX_LOCAL_RADIUS: usize = 3;

    pub const FRACTAL_NUM_SCALES: usize = 8;
    pub const FRACTAL_BOOTSTRAP_SAMPLES_MIN: usize = 100;
    pub const FRACTAL_BOOTSTRAP_SAMPLES_MAX: usize = 500;
    pub const FRACTAL_QUALITY_THRESHOLD: f64 = 0.3;
    pub const FRACTAL_CONFIDENCE_LEVEL: f64 = 0.95;

    pub const MK_ALPHA: f64 = 0.05;

    pub const POPULATION_GRID_SIZE: f64 = 0.002;
    pub const POPULATION_PERSONS_PER_ROOM: f64 = 4.5;
    pub const POPULATION_ALLOMETRIC_EXPONENT: f64 = 0.85;
    pub const POPULATION_IDW_POWER: f64 = 2.0;

    pub const DEFENSE_NUM_SAMPLE_POINTS: usize = 36;
    pub const DEFENSE_VISIBILITY_RADIUS_KM: f64 = 2.0;
    pub const DEFENSE_NUM_ATTACK_ROUTES: usize = 6;
    pub const DEFENSE_WALL_SEGMENTS: usize = 24;

    pub const LAND_USE_NUM_PERIODS: usize = 8;
    pub const LAND_USE_DECAY_RATE: f64 = 0.7;
}
