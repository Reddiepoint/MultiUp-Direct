use eframe::egui;
use egui::Direction::TopDown;
use egui::Align2;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use std::time::Duration;

pub fn new_toasts() -> Toasts {
    Toasts::new()
        .anchor(Align2::RIGHT_TOP, (10.0, 10.0))
        .direction(TopDown)
}

pub fn new_toast(toasts: &mut Toasts, toast_kind: ToastKind, message: &str, duration: Option<Option<Duration>>, show_progress: Option<bool>) {
    toasts.add(Toast {
        kind: toast_kind,
        text: message.into(),
        options: ToastOptions::default()
            .duration(duration.unwrap_or(Some(Duration::from_secs(3))))
            .show_progress(show_progress.unwrap_or(true)),
        style: Default::default(),
    });
}

pub fn new_info_toast(toasts: &mut Toasts, message: &str, duration: Option<Option<Duration>>, show_progress: Option<bool>) {
    new_toast(toasts, ToastKind::Info, message, duration, show_progress);
}

pub fn new_warning_toast(toasts: &mut Toasts, message: &str, duration: Option<Option<Duration>>, show_progress: Option<bool>) {
    new_toast(toasts, ToastKind::Warning, message, duration, show_progress);
}

pub fn new_error_toast(toasts: &mut Toasts, message: &str, duration: Option<Option<Duration>>, show_progress: Option<bool>) {
    new_toast(toasts, ToastKind::Error, message, duration, show_progress);
}
pub fn new_success_toast(toasts: &mut Toasts, message: &str, duration: Option<Option<Duration>>, show_progress: Option<bool>) {
    new_toast(toasts, ToastKind::Success, message, duration, show_progress);
}
