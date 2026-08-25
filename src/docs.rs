// Структура для виджета документации и дефолтное состояние
#[derive(Default)]
pub struct DocWidget {
    pub is_open: bool,
    selected_section: DocSection,
}

// Секции докумментации
#[derive(PartialEq, Eq, Default)]
pub enum DocSection {
    #[default]
    Overview,
    Principles,
    Blocks,
}

impl DocWidget {
    pub fn render_docs(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("doc_sidebar")
            .resizable(true)
            .default_size(180.0)
            .size_range(120.0..=300.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("doc_sidebar_scroll")
                    .show(ui, |ui| {
                        ui.heading("Разделы");
                        ui.add_space(8.0);

                        ui.selectable_value(
                            &mut self.selected_section,
                            DocSection::Overview,
                            "Обзор",
                        );
                        ui.selectable_value(
                            &mut self.selected_section,
                            DocSection::Principles,
                            "Принцип работы",
                        );
                        ui.selectable_value(
                            &mut self.selected_section,
                            DocSection::Blocks,
                            "Компоненты",
                        );
                    });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("doc_content_scroll")
                .show(ui, |ui| match self.selected_section {
                    DocSection::Overview => {
                        ui.heading("Обзор системы");
                        ui.add_space(8.0);
                        ui.label("Это модуль документации");
                    }
                    DocSection::Principles => {}
                    DocSection::Blocks => {}
                });
        });
    }
}
