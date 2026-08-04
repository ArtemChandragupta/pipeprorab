const G_GRAV: f64 = 9.81;
const RHO: f64 = 1000.0;
const NU: f64 = 1e-6;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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
