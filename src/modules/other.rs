use eframe::egui::Ui;
use std::fs::File;
use std::io::{BufWriter, Write};

pub fn copy_to_clipboard(ui: &Ui, text: String) {
    ui.output_mut(|output| output.copied_text = text);
}

pub fn save_to_file(filename: &str, text: &str) -> Result<(), String> {
    let file = match File::create(filename) {
        Ok(file) => { file }
        Err(error) => {
            eprintln!("Error creating file: {}", error);
            return Err(error.to_string());
        }
    };
    let mut writer = BufWriter::new(file);
    match writer.write_all(text.as_bytes()) {
        Ok(_) => {
            Ok(())
        }
        Err(error) => {
            eprintln!("Error writing to file: {}", error);
            Err(error.to_string())
        }
    }
}
