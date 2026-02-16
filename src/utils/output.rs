// Colored terminal output helpers
use colored::Colorize;

pub fn print_success(msg: &str) {
    println!("{} {}", "PASS".green(), msg);
}

pub fn print_error(msg: &str) {
    eprintln!("{} {}", "FAIL".red(), msg);
}

pub fn print_info(msg: &str) {
    println!("{} {}", "ℹ️ ".blue(), msg);
}

pub fn print_warning(msg: &str) {
    println!("{} {}", "⚠️ ".yellow(), msg);
}
