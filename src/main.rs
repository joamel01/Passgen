#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod generator;

use eframe::egui;
use generator::{ApiFormat, Mode, Options};

struct App {
    options: Options,
    output: String,
    show_output: bool,
    status: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            options: Options::default(),
            output: String::new(),
            show_output: false,
            status: "Välj inställningar och tryck Generera.".into(),
        }
    }
}

impl App {
    fn generate(&mut self) {
        match generator::generate(&self.options) {
            Ok(value) => {
                self.output = value;
                self.status = "Ny kod skapad med Windows slumpkälla.".into();
            }
            Err(error) => self.status = error,
        }
    }

    fn mode_name(&self) -> &'static str {
        match self.options.mode {
            Mode::Password => "Lösenord",
            Mode::Pin => "Sifferkod",
            Mode::ApiKey => "API-nyckel",
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(12.0);
            ui.heading("Nyckelverkstad");
            ui.label("Skapa lösenord, sifferkoder och API-nycklar lokalt på datorn.");
            ui.add_space(18.0);

            ui.horizontal(|ui| {
                ui.label("Typ");
                egui::ComboBox::from_id_salt("mode")
                    .selected_text(self.mode_name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.options.mode, Mode::Password, "Lösenord");
                        ui.selectable_value(&mut self.options.mode, Mode::Pin, "Sifferkod (PIN)");
                        ui.selectable_value(&mut self.options.mode, Mode::ApiKey, "API-nyckel");
                    });
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Längd");
                ui.add(egui::DragValue::new(&mut self.options.length).range(4..=256).speed(1));
                ui.label("tecken");
            });

            match self.options.mode {
                Mode::Password => {
                    ui.add_space(12.0);
                    ui.label("Tecken som får användas");
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut self.options.upper, "Versaler A–Z");
                        ui.checkbox(&mut self.options.lower, "Gemener a–z");
                        ui.checkbox(&mut self.options.digits, "Siffror 0–9");
                        ui.checkbox(&mut self.options.symbols, "Specialtecken");
                    });
                    ui.checkbox(&mut self.options.avoid_ambiguous, "Undvik lättförväxlade tecken (O, 0, I, l, 1, |)");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Snabbval");
                        if ui.button("Bokstäver").clicked() {
                            self.options.upper = true;
                            self.options.lower = true;
                            self.options.digits = false;
                            self.options.symbols = false;
                        }
                        if ui.button("Bokstäver + siffror").clicked() {
                            self.options.upper = true;
                            self.options.lower = true;
                            self.options.digits = true;
                            self.options.symbols = false;
                        }
                        if ui.button("Alla tecken").clicked() {
                            self.options.upper = true;
                            self.options.lower = true;
                            self.options.digits = true;
                            self.options.symbols = true;
                        }
                    });
                }
                Mode::Pin => {
                    ui.add_space(12.0);
                    ui.label("Sifferkoden innehåller enbart 0–9.");
                    ui.horizontal(|ui| {
                        ui.label("Snabbval");
                        for size in [4, 6, 8, 12] {
                            if ui.button(format!("{size} siffror")).clicked() {
                                self.options.length = size;
                            }
                        }
                    });
                }
                Mode::ApiKey => {
                    ui.add_space(12.0);
                    ui.label("Format");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.options.api_format, ApiFormat::UrlSafe, "URL-säkert (A–Z, a–z, 0–9, - _)");
                        ui.radio_value(&mut self.options.api_format, ApiFormat::Hex, "Hex (0–9, a–f)");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Snabbval");
                        for size in [32, 48, 64, 128] {
                            if ui.button(format!("{size} tecken")).clicked() {
                                self.options.length = size;
                            }
                        }
                    });
                }
            }

            ui.add_space(22.0);
            if ui.add_sized([140.0, 34.0], egui::Button::new("Generera")).clicked() {
                self.generate();
            }
            ui.add_space(12.0);
            ui.label("Resultat");
            ui.horizontal(|ui| {
                let edit = egui::TextEdit::singleline(&mut self.output)
                    .password(!self.show_output)
                    .desired_width(f32::INFINITY);
                ui.add_enabled(false, edit);
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_output, "Visa");
                if ui.add_enabled(!self.output.is_empty(), egui::Button::new("Kopiera")).clicked() {
                    ctx.copy_text(self.output.clone());
                    self.status = "Kopierat till urklipp.".into();
                }
                if ui.add_enabled(!self.output.is_empty(), egui::Button::new("Rensa")).clicked() {
                    self.output.clear();
                    self.status = "Resultatet rensat.".into();
                }
            });
            ui.add_space(10.0);
            ui.label(&self.status);
            ui.add_space(8.0);
            ui.small("Tips: urklippet kan läsas av andra program. Rensa det efter att du klistrat in hemligheten.");
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Nyckelverkstad")
            .with_inner_size([740.0, 520.0])
            .with_min_inner_size([620.0, 460.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Nyckelverkstad",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(App::default()))
        }),
    )
}
