use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    // app state
    let mut name = "Arthur".to_owned();
    let mut age = 42;

    eframe::run_simple_native("app_name", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui|{
            ui.heading("my app heading");
            ui.horizontal(|ui|{
                let name_label = ui.label("name: ");
                ui.text_edit_singleline(&mut name)
                .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                age += 1;
            }
            ui.label(format!("Hi '{name}', age {age}"));
        });
    })
}
