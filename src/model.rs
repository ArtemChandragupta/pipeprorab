use egui_snarl::{NodeId, Snarl};
use std::f64::consts::PI;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    f64,
};

const G_GRAV: f64 = 9.81;
const RHO: f64 = 1000.0;
const NU: f64 = 1e-6;

// Структура для графического отображения - с неё всё начинается
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

// --- СТРУКТУРЫ ДЛЯ РАСЧЕТА ---
// Гидравлическое состояние элемента
#[derive(Default, Debug, Clone)]
pub struct HydraulicState {
    pub k: f64,
    pub q: f64,
    pub p_in: f64,
    pub p_out: f64,
}

// Имя, тип и состояние элемента
#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub kind: ElementKind,
    pub state: HydraulicState,
}

// Различные типы элементов для расчета, чтобы построить из них вложенный граф для рекурсии. Здесь нет насоса, зато есть параллельность и последовательность - всё для парсинга
#[derive(Debug, Clone)]
pub enum ElementKind {
    Pipe { l: f64, d: f64, r: f64 },
    Fitting { d: f64, zeta: f64 },
    Series(Vec<Component>),
    Parallel(Vec<Component>),
}

impl Component {
    fn new(name: impl Into<String>, kind: ElementKind) -> Self {
        Self {
            name: name.into(),
            kind,
            state: HydraulicState::default(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self.kind {
            ElementKind::Pipe { .. } => "pipe",
            ElementKind::Fitting { .. } => "fitting",
            ElementKind::Series(_) => "series",
            ElementKind::Parallel(_) => "parallel",
        }
    }
}

// Насос. Превращает три точки в кривую.
pub struct Pump {
    a: f64,
    b: f64,
    c: f64,
}
impl Pump {
    pub fn from_points(p: &[[f64; 2]; 3]) -> Result<Self, &'static str> {
        let ((q1, h1), (q2, h2), (q3, h3)) =
            ((p[0][0], p[0][1]), (p[1][0], p[1][1]), (p[2][0], p[2][1]));
        let denom = (q1 - q2) * (q1 - q3) * (q2 - q3);
        if denom.abs() < 1e-12 {
            return Err("Расход Q для точек должен различаться.");
        }

        Ok(Self {
            a: (h1 * (q2 - q3) - h2 * (q1 - q3) + h3 * (q1 - q2)) / denom,
            b: (-h1 * (q2 * q2 - q3 * q3) + h2 * (q1 * q1 - q3 * q3) - h3 * (q1 * q1 - q2 * q2))
                / denom,
            c: (h1 * q2 * q3 * (q2 - q3) - h2 * q1 * q3 * (q1 - q3) + h3 * q1 * q2 * (q1 - q2))
                / denom,
        })
    }

    fn head(&self, q: f64) -> f64 {
        self.a * q.powi(2) + self.b * q + self.c
    }
}

// --- ЛОГИКА РАСЧЕТОВ ---
pub fn get_static_pressure(comp: &Component) -> f64 {
    match &comp.kind {
        ElementKind::Series(elems) => elems.iter().map(get_static_pressure).sum(),
        ElementKind::Parallel(branches) => branches.first().map_or(0.0, get_static_pressure),
        _ => 0.0,
    }
}

// Расчет эквивалентного сопротивления
pub fn update_k(comp: &mut Component, q_in: f64) -> f64 {
    let q = q_in.abs().max(1e-9);

    let k = match &mut comp.kind {
        ElementKind::Pipe { l, d, r } => {
            let v = 4.0 * q / (PI * d.powi(2));
            let re = (v * *d) / NU;
            let lam = if re < 2300.0 {
                64.0 / re.max(1e-3)
            } else if re < 4000.0 {
                0.3164 / re.powf(0.25)
            } else {
                0.11 * ((*r / *d) + (68.0 / re)).powf(0.25)
            };
            (8.0 * lam * *l * RHO) / (PI.powi(2) * d.powi(5))
        }
        ElementKind::Fitting { d, zeta } => (8.0 * *zeta * RHO) / (PI.powi(2) * d.powi(4)),
        ElementKind::Series(elems) => elems.iter_mut().map(|e| update_k(e, q)).sum(),
        ElementKind::Parallel(branches) => {
            let inv_sqrts: Vec<f64> = branches
                .iter()
                .map(|b| {
                    if b.state.k > 0.0 {
                        1.0 / b.state.k.sqrt()
                    } else {
                        1e9
                    }
                })
                .collect();
            let total_inv: f64 = inv_sqrts.iter().sum();

            let k_new: Vec<f64> = branches
                .iter_mut()
                .zip(&inv_sqrts)
                .map(|(b, &inv)| update_k(b, q * (inv / total_inv)))
                .collect();

            if k_new.contains(&0.0) {
                0.0
            } else {
                1.0 / k_new.iter().map(|k| 1.0 / k.sqrt()).sum::<f64>().powi(2)
            }
        }
    };
    comp.state.k = k;
    k
}

// Финальный расчет падения давления на элементе
pub fn calc_flow_pressure(comp: &mut Component, q_in: f64, p_in: f64) -> f64 {
    comp.state.q = q_in;
    comp.state.p_in = p_in;
    let k = comp.state.k;

    let p_out = match &mut comp.kind {
        ElementKind::Pipe { .. } | ElementKind::Fitting { .. } => p_in - k * q_in.powi(2),
        ElementKind::Series(elems) => elems
            .iter_mut()
            .fold(p_in, |p, e| calc_flow_pressure(e, q_in, p)),
        ElementKind::Parallel(branches) => {
            let static_drop = get_static_pressure(branches.first().unwrap());
            let out = p_in - k * q_in.powi(2) - static_drop;

            let invs: Vec<f64> = branches
                .iter()
                .map(|b| {
                    if b.state.k > 0.0 {
                        1.0 / b.state.k.sqrt()
                    } else {
                        f64::INFINITY
                    }
                })
                .collect();
            let has_inf = invs.contains(&f64::INFINITY);
            let total_inv = invs.iter().filter(|&&x| x != f64::INFINITY).sum::<f64>();

            for (b, &inv) in branches.iter_mut().zip(&invs) {
                let share = if has_inf {
                    if inv == f64::INFINITY { 1.0 } else { 0.0 }
                } else {
                    inv / total_inv
                };
                calc_flow_pressure(b, q_in * share, p_in);
            }
            out
        }
    };
    comp.state.p_out = p_out;
    p_out
}

// Поиск рабочей точки системы
pub fn solve_operating_point(
    pump: &Pump,
    k_total: f64,
    h_static: f64,
) -> Result<(f64, f64), &'static str> {
    let (a, b, c) = (
        pump.a - (k_total / (RHO * G_GRAV)),
        pump.b,
        pump.c - h_static,
    );

    let d = b * b - 4.0 * a * c;
    if d < 0.0 {
        return Err("Нет пересечения характеристики насоса и сети");
    }

    let q_op = ((-b + d.sqrt()) / (2.0 * a))
        .max((-b - d.sqrt()) / (2.0 * a))
        .max(0.0);
    Ok((q_op, pump.head(q_op)))
}

// --- КОНВЕРТАЦИЯ ГРАФА В МОДЕЛЬ (Написано Gemini) ---
pub fn build_model(snarl: &Snarl<PipeNode>) -> Result<([[f64; 2]; 3], Component), &'static str> {
    let (pump_node, pump_points) = snarl
        .node_ids()
        .find_map(|(id, n)| {
            if let PipeNode::Pump { points } = n {
                Some((
                    id,
                    [
                        [points[0].0, points[0].1],
                        [points[1].0, points[1].1],
                        [points[2].0, points[2].1],
                    ],
                ))
            } else {
                None
            }
        })
        .ok_or("Насос не найден в графе!")?;

    let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for (out_pin, in_pin) in snarl.wires() {
        adj.entry(out_pin.node).or_default().push(in_pin.node);
    }

    fn find_conv(
        backbone: NodeId,
        branches: &[NodeId],
        adj: &HashMap<NodeId, Vec<NodeId>>,
    ) -> Option<NodeId> {
        if branches.is_empty() {
            return None;
        }
        let mut reach = branches
            .iter()
            .map(|&start| {
                let (mut vis, mut q) = (HashSet::from([start]), VecDeque::from([start]));
                while let Some(n) = q.pop_front() {
                    for &nxt in adj.get(&n).unwrap_or(&vec![]) {
                        if vis.insert(nxt) {
                            q.push_back(nxt);
                        }
                    }
                }
                vis
            })
            .collect::<Vec<_>>();

        let mut common = reach.pop().unwrap();
        for set in reach {
            common.retain(|n| set.contains(n));
        }
        if common.is_empty() {
            return None;
        }

        let (mut vis, mut q) = (HashSet::from([backbone]), VecDeque::from([backbone]));
        while let Some(n) = q.pop_front() {
            if n != backbone && common.contains(&n) {
                return Some(n);
            }
            for &nxt in adj.get(&n).unwrap_or(&vec![]) {
                if vis.insert(nxt) {
                    q.push_back(nxt);
                }
            }
        }
        None
    }

    fn process_chain(
        mut curr: NodeId,
        end_at: Option<NodeId>,
        snarl: &Snarl<PipeNode>,
        adj: &HashMap<NodeId, Vec<NodeId>>,
    ) -> Vec<Component> {
        let mut elems = Vec::new();
        while Some(curr) != end_at {
            match &snarl[curr] {
                PipeNode::Pipe {
                    length,
                    diameter,
                    roughness,
                } => elems.push(Component::new(
                    format!("Труба L={length:.1} D={diameter:.3}"),
                    ElementKind::Pipe {
                        l: *length,
                        d: *diameter,
                        r: *roughness,
                    },
                )),
                PipeNode::Fitting { diameter, zeta } => elems.push(Component::new(
                    format!("МС D={diameter:.3} ξ={zeta:.2}"),
                    ElementKind::Fitting {
                        d: *diameter,
                        zeta: *zeta,
                    },
                )),
                PipeNode::Pump { .. } => {}
            }
            match adj.get(&curr).map(|v| v.as_slice()) {
                Some([next]) => curr = *next,
                Some(nexts) => {
                    let conv = find_conv(curr, nexts, adj);
                    let branches = nexts
                        .iter()
                        .map(|&start| {
                            Component::new(
                                "Ветвь",
                                ElementKind::Series(process_chain(start, conv, snarl, adj)),
                            )
                        })
                        .collect();
                    elems.push(Component::new(
                        if conv.is_some() {
                            "Разветвление"
                        } else {
                            "Разветвление (без слияния)"
                        },
                        ElementKind::Parallel(branches),
                    ));
                    if let Some(c) = conv {
                        curr = c;
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }
        elems
    }

    let initial = adj
        .get(&pump_node)
        .and_then(|n| n.first())
        .map_or(vec![], |&start| process_chain(start, None, snarl, &adj));
    Ok((
        pump_points,
        Component::new("Магистраль", ElementKind::Series(initial)),
    ))
}
