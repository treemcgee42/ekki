use std::sync::Mutex;


/// The actual type/instance provided to `log`. Since the functions for logging take an 
/// immutable reference to the instance, we opt to have this struct be a singleton which 
/// mutates a different global static (`LogBuffer`).
struct EguiLogger;
static EGUI_LOGGER: EguiLogger = EguiLogger {};

impl log::Log for EguiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let log_buffer = LOG_BUFFER.lock().unwrap();

        if metadata.level() > log_buffer.log_level_filter {
            return false;
        }

        return true;
    }

    fn log(&self, record: &log::Record) {
        let mut log_lock = LOG_BUFFER.lock();
        let buffer = log_lock.as_mut().unwrap();

        if buffer.is_paused { return; }

        let wgpu_enabled = buffer.filter.wgpu.enabled;
        let starts_with_wgpu = record.metadata().target().starts_with("wgpu");
        if !wgpu_enabled && starts_with_wgpu {
            return;
        } else if wgpu_enabled && starts_with_wgpu {
            if record.metadata().level() > buffer.filter.wgpu.log_level_filter {
                return;
            }
        }

        let winit_enabled = buffer.filter.winit.enabled;
        let starts_with_winit = record.metadata().target().starts_with("winit");
        if !winit_enabled && starts_with_winit {
            return;
        } else if winit_enabled && starts_with_winit {
            if record.metadata().level() > buffer.filter.winit.log_level_filter {
                return;
            }
        }

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
    filter: LogFilter,
    is_paused: bool,
}
static LOG_BUFFER: Mutex<LogBuffer> = Mutex::new(LogBuffer::new(100, log::LevelFilter::Info));

impl LogBuffer {
    const fn new(max_lines: usize, log_level_filter: log::LevelFilter) -> Self {
        Self {
            lines: Vec::new(),
            max_lines,
            log_level_filter,
            filter: LogFilter::const_default(),
            is_paused: false,
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

struct LibraryLogFilter {
    enabled: bool,
    log_level_filter: log::LevelFilter,
}

impl LibraryLogFilter {
    const fn const_default() -> Self {
        Self {
            enabled: false,
            log_level_filter: log::LevelFilter::Error,
        }
    }
}

struct LogFilter {
    wgpu: LibraryLogFilter,
    winit: LibraryLogFilter,
}

impl LogFilter {
    const fn const_default() -> Self {
        Self {
            wgpu: LibraryLogFilter::const_default(),
            winit: LibraryLogFilter::const_default(),
        }
    }
}

pub fn init(log_level_filter: log::LevelFilter) -> Result<(), log::SetLoggerError> {
    LOG_BUFFER.lock().unwrap().log_level_filter = log_level_filter;
    log::set_logger(&EGUI_LOGGER)?;
    Ok(log::set_max_level(log_level_filter))
}

pub fn draw_egui_console_menu(ui: &mut egui::Ui) {
    egui::menu::bar(ui, |ui| {
        let mut log_buffer = LOG_BUFFER.lock().unwrap();

        ui.menu_button("Edit", |ui| {
            let mut paused = log_buffer.is_paused.clone();
            ui.checkbox(&mut paused, "Paused");
            log_buffer.is_paused = paused;

            if ui.button("Clear").clicked() {
                log_buffer.clear();
            }
        });

        ui.menu_button("Filter", |ui| {
            ui.menu_button("Libraries", |ui| {
                ui.menu_button("wgpu", |ui| {
                    let mut enabled = log_buffer.filter.wgpu.enabled.clone();
                    let mut selected_level_filter_value = log_buffer.filter.wgpu.log_level_filter.clone();

                    ui.checkbox(&mut enabled, "Enabled");
                    draw_egui_log_level_options(&mut selected_level_filter_value, ui);

                    log_buffer.filter.wgpu.enabled = enabled;
                    log_buffer.filter.wgpu.log_level_filter = selected_level_filter_value;
                });

                ui.menu_button("winit", |ui| {
                    let mut enabled = log_buffer.filter.winit.enabled.clone();
                    let mut selected_level_filter_value = log_buffer.filter.winit.log_level_filter.clone();

                    ui.checkbox(&mut enabled, "Enabled");
                    draw_egui_log_level_options(&mut selected_level_filter_value, ui);

                    log_buffer.filter.winit.enabled = enabled;
                    log_buffer.filter.winit.log_level_filter = selected_level_filter_value;
                });
            });

            ui.menu_button("Level", |ui| {
                let mut selected_level_filter_value = log_buffer.log_level_filter.clone();
                draw_egui_log_level_options(&mut selected_level_filter_value, ui);

                log_buffer.set_log_level_filter(selected_level_filter_value);
            });
        });
    });
}

fn draw_egui_log_level_options(selected_level_filter_value: &mut log::LevelFilter, ui: &mut egui::Ui) {
    ui.radio_value(selected_level_filter_value, log::LevelFilter::Error, "Error");
    ui.radio_value(selected_level_filter_value, log::LevelFilter::Warn, "Warn");
    ui.radio_value(selected_level_filter_value, log::LevelFilter::Info, "Info");
    ui.radio_value(selected_level_filter_value, log::LevelFilter::Debug, "Debug");
    ui.radio_value(selected_level_filter_value, log::LevelFilter::Trace, "Trace");
}

// Parameters:
//  - `ui`: provided by the egui region the lines are being rendered into.
pub fn draw_egui_logging_lines(ui: &mut egui::Ui) {
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

