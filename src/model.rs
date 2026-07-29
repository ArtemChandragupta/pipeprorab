pub enum PipeNode {
    Pipe {
        length: f64,
        diameter: f64,
        roughness: f64,
    },
    Fitting {
        diameter: f64,
        zeta: f64,
    },
    Pump {
        points: [(f64, f64); 3],
    },
}
