use crate::model::{CalculationResult, G_GRAV, PipeNode, PipeNodeKind, RHO};
use egui_plot::{Legend, Line, Plot, PlotPoints, Points};
use egui_snarl::Snarl;

fn interpolate_quadratic(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], q: f64) -> f64 {
    let (q0, h0) = (p0[0], p0[1]);
    let (q1, h1) = (p1[0], p1[1]);
    let (q2, h2) = (p2[0], p2[1]);

    let l0 = (q - q1) * (q - q2) / ((q0 - q1) * (q0 - q2));
    let l1 = (q - q0) * (q - q2) / ((q1 - q0) * (q1 - q2));
    let l2 = (q - q0) * (q - q1) / ((q2 - q0) * (q2 - q1));

    h0 * l0 + h1 * l1 + h2 * l2
}

pub fn draw_hq_plot(ui: &mut egui::Ui, res: &CalculationResult, snarl: &Snarl<PipeNode>) {
    ui.heading("Характеристики системы (Q-H)");

    let plot = Plot::new("hq_plot")
        .height(300.0)
        .x_axis_label("Расход Q (м³/с)")
        .y_axis_label("Напор H (м)")
        .legend(Legend::default())
        .include_x(0.0)
        .allow_zoom(true)
        .include_y(0.0);

    plot.show(ui, |plot_ui| {
        let op_q = res.q_op;
        let op_h = res.h_op;

        let q_max = (op_q * 1.5).max(0.01);
        let steps = 100;

        // 1. Построение параболы сети
        let mut net_points = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let q = q_max * (i as f64) / (steps as f64);
            let h = res.h_static + res.k_total * q * q / (RHO * G_GRAV);
            net_points.push([q, h]);
        }

        plot_ui.line(
            Line::new("Кривая сети", PlotPoints::from(net_points)).color(egui::Color32::BLUE),
        );

        // 2. Построение характеристики насоса по трем точкам
        for node in snarl.nodes() {
            if let PipeNodeKind::Pump { points } = &node.kind {
                let mut pts: Vec<[f64; 2]> = points.iter().map(|&(q, h)| [q, h]).collect();

                pts.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());

                if pts.len() >= 3 {
                    let p0 = pts[0];
                    let p1 = pts[1];
                    let p2 = pts[2];

                    let min_q = 0.0;
                    let max_q = pts.last().unwrap()[0] * 1.15;

                    let mut dense_pump_pts = Vec::with_capacity(steps + 1);
                    for i in 0..=steps {
                        let q = min_q + (max_q - min_q) * (i as f64) / (steps as f64);
                        let h = interpolate_quadratic(p0, p1, p2, q);
                        dense_pump_pts.push([q, h]);
                    }

                    // Линия характеристики насоса
                    plot_ui.line(
                        Line::new("Кривая насоса", PlotPoints::from(dense_pump_pts))
                            .color(egui::Color32::RED),
                    );

                    // Опорные точки насоса
                    plot_ui.points(
                        Points::new("Точки насоса", PlotPoints::from(pts))
                            .radius(3.5)
                            .color(egui::Color32::RED),
                    );
                } else {
                    plot_ui.line(
                        Line::new("Насос (ломаная)", PlotPoints::from(pts))
                            .color(egui::Color32::RED),
                    );
                }
            }
        }

        // 3. Рабочая точка
        plot_ui.points(
            Points::new("Рабочая точка", PlotPoints::from(vec![[op_q, op_h]]))
                .radius(5.0)
                .color(egui::Color32::GREEN),
        );
    });
}
