use libproto::services::LogMarkerColor;

pub const RESET: &'static str = "\x1B[0m";  // Text Reset
pub const BLACK: &'static str = "\x1B[0;30m";
pub const RED: &'static str = "\x1B[0;31m";
pub const GREEN: &'static str = "\x1B[0;32m";
pub const YELLOW: &'static str = "\x1B[0;33m";
pub const BLUE: &'static str = "\x1B[0;34m";
pub const MAGENTA: &'static str = "\x1B[0;35m";
pub const CYAN: &'static str = "\x1B[0;36m";
pub const WHITE: &'static str = "\x1B[0;37m";
pub const BRIGHT_BLACK: &'static str = "\x1B[0;90m";
pub const BRIGHT_RED: &'static str = "\x1B[0;91m";
pub const BRIGHT_GREEN: &'static str = "\x1B[0;92m";
pub const BRIGHT_YELLOW: &'static str = "\x1B[0;93m";
pub const BRIGHT_BLUE: &'static str = "\x1B[0;94m";
pub const BRIGHT_MAGENTA: &'static str = "\x1B[0;95m";
pub const BRIGHT_CYAN: &'static str = "\x1B[0;96m";
pub const BRIGHT_WHITE: &'static str = "\x1B[0;97m";

pub fn log_marker_ansi_color(color: LogMarkerColor) -> &'static str {
    match color {
        LogMarkerColor::Black => BLACK,
        LogMarkerColor::Red => RED,
        LogMarkerColor::Green => GREEN,
        LogMarkerColor::Yellow => YELLOW,
        LogMarkerColor::Blue => BLUE,
        LogMarkerColor::Magenta => MAGENTA,
        LogMarkerColor::Cyan => CYAN,
        LogMarkerColor::White => WHITE,
        LogMarkerColor::BrightBlack => BRIGHT_BLACK,
        LogMarkerColor::BrightRed => BRIGHT_RED,
        LogMarkerColor::BrightGreen => BRIGHT_GREEN,
        LogMarkerColor::BrightYellow => BRIGHT_YELLOW,
        LogMarkerColor::BrightBlue => BRIGHT_BLUE,
        LogMarkerColor::BrightMagenta => BRIGHT_MAGENTA,
        LogMarkerColor::BrightCyan => BRIGHT_CYAN,
        LogMarkerColor::BrightWhite => BRIGHT_WHITE,
    }
}

