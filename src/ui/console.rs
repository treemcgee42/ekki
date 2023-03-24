use std::sync::Mutex;


/// The actual type/instance provided to `log`. Since the functions for logging take an 
/// immutable reference to the instance, we opt to have this struct be a singleton which 
/// mutates a different global static (`LogBuffer`).
struct EguiLogger;
static EGUI_LOGGER: EguiLogger = EguiLogger {};

impl log::Log for EguiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= LOG_BUFFER.lock().unwrap().log_level_filter
    }

    fn log(&self, record: &log::Record) {
        let mut log_lock = LOG_BUFFER.lock();
        let buffer = log_lock.as_mut().unwrap();

        let line = format!("target: {}, args: {}", record.target(), record.args().to_string());
        buffer.lines.push(line);

        let max_lines = buffer.max_lines;
        buffer.lines.reverse();
        buffer.lines.truncate(max_lines);
        buffer.lines.reverse();
    }

    fn flush(&self) {}
}

/// This is the type/instance that is logger (`EguiLogger`) writes to, and from which the 
/// UI reads from.
struct LogBuffer {
    lines: Vec<String>,
    max_lines: usize,
    log_level_filter: log::LevelFilter,
}
static LOG_BUFFER: Mutex<LogBuffer> = Mutex::new(LogBuffer::new(100, log::LevelFilter::Info));

impl LogBuffer {
    const fn new(max_lines: usize, log_level_filter: log::LevelFilter) -> Self {
        Self {
            lines: Vec::new(),
            max_lines,
            log_level_filter,
        }
    }

    fn set_log_level_filter(&mut self, log_level_filter: log::LevelFilter) {
        log::set_max_level(log_level_filter);
        self.log_level_filter = log_level_filter;
    }

    fn clear(&mut self) {
        self.lines.drain(..);
    }
}

pub fn init(log_level_filter: log::LevelFilter) -> Result<(), log::SetLoggerError> {
    LOG_BUFFER.lock().unwrap().log_level_filter = log_level_filter;
    log::set_logger(&EGUI_LOGGER)?;
    Ok(log::set_max_level(log_level_filter))
}

pub fn draw_egui_console_menu(ui: &mut egui::Ui) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("Edit", |ui| {
            if ui.button("Clear Logs").clicked() {
                LOG_BUFFER.lock().unwrap().clear();
            }
        });

        ui.menu_button("Level", |ui| {
            let mut log_buffer = LOG_BUFFER.lock().unwrap();

            let mut selected_level_filter_value = log_buffer.log_level_filter.clone();
            ui.radio_value(&mut selected_level_filter_value, log::LevelFilter::Error, "Error");
            ui.radio_value(&mut selected_level_filter_value, log::LevelFilter::Warn, "Warn");
            ui.radio_value(&mut selected_level_filter_value, log::LevelFilter::Info, "Info");
            ui.radio_value(&mut selected_level_filter_value, log::LevelFilter::Debug, "Debug");
            ui.radio_value(&mut selected_level_filter_value, log::LevelFilter::Trace, "Trace");

            log_buffer.set_log_level_filter(selected_level_filter_value);
        });

    });
}

// Parameters:
//  - `ui`: provided by the egui region the lines are being rendered into.
pub fn draw_egui_logging_lines(ui: &mut egui::Ui) {
    ui.label("Rows enter from the bottom, we want the scroll handle to start and stay at bottom unless moved");

    ui.add_space(4.0);

    let text_style = egui::TextStyle::Body;
    let row_height = ui.text_style_height(&text_style);
    let buffer = LOG_BUFFER.lock().unwrap();
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, true]) // auto shrink vertically but not horizontally
        .show_rows(
        ui,
        row_height,
        buffer.lines.len(),
        |ui, row_range| {
            for row in row_range {
                ui.label(buffer.lines.get(row).unwrap());
            }
        },
    );

    ui.ctx().request_repaint();
}

