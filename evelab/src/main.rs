use eframe::egui;
use egui::containers::Frame;
use emath::{Pos2, Rect};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    // app state
    let mut name = "Arthur".to_owned();
    let mut age = 42;
    let points = vec![Pos2::new(0.1, 0.2), Pos2::new(0.3, 0.4)];

    eframe::run_simple_native("app_name", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("my app heading");
            ui.horizontal(|ui| {
                let name_label = ui.label("name: ");
                ui.text_edit_singleline(&mut name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                age += 1;
            }
            ui.label(format!("Hi '{name}', age {age}"));
            Frame::canvas(ui.style()).show(ui, |ui| {
                ui.ctx().request_repaint();

                // let desired_size = ui.available_width() * vec2(1.0, 0.35);
                //let (_id, rect) = ui.allocate_space(desired_size);
                let rect = ui.available_rect_before_wrap();

                let to_screen = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0),
                    rect,
                );

                let mut shapes = vec![];

                let points: Vec<Pos2> = points.iter().map(|p| to_screen * *p).collect();

                let radius = 10.0;
                for point in &points {
                    shapes.push(egui::epaint::Shape::circle_filled(*point, radius, egui::Color32::from_rgb(255,255,255)));
                }
                ui.painter().extend(shapes);
            });
        });
    })
}
