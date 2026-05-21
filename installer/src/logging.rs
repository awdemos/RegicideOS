/// ANSI colour codes for terminal output formatting.
pub struct Colours;

impl Colours {
    pub const RED: &'static str = "\x1b[31m";
    pub const YELLOW: &'static str = "\x1b[33m";
    pub const BLUE: &'static str = "\x1b[34m";
    pub const ENDC: &'static str = "\x1b[m";
}

/// Prints an error message and exits the process with code 1.
pub fn die(message: &str) -> ! {
    // Sanitize error message to prevent information disclosure
    let sanitized = sanitize_error_message(message);
    eprintln!("{}[ERROR]{} {}", Colours::RED, Colours::ENDC, sanitized);
    std::process::exit(1);
}

/// Sanitizes error messages to prevent information disclosure.
pub fn sanitize_error_message(message: &str) -> String {
    // Remove potentially sensitive information
    let sensitive_patterns = [
        r"/home/[^/\s]+",             // Home directory paths
        r"/root/[^/\s]+",             // Root directory paths
        r"/tmp/[^/\s]+",              // Temp file paths
        r"password[^=\s]*=\s*[^\s]+", // Passwords in error messages
        r"token[^=\s]*=\s*[^\s]+",    // Tokens in error messages
        r"key[^=\s]*=\s*[^\s]+",      // Keys in error messages
        r"secret[^=\s]*=\s*[^\s]+",   // Secrets in error messages
    ];

    let mut sanitized = message.to_string();

    for pattern in &sensitive_patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            sanitized = regex.replace_all(&sanitized, "[REDACTED]").to_string();
        }
    }

    // Remove full paths, keep only filenames
    let path_regex = regex::Regex::new(r"/([^/\s]+/)+([^/\s]+)").unwrap();
    sanitized = path_regex.replace_all(&sanitized, "[PATH]/$2").to_string();

    // Limit error message length to prevent log flooding
    if sanitized.len() > 200 {
        sanitized.truncate(197);
        sanitized.push_str("...");
    }

    sanitized
}

/// Prints an informational message to stdout.
pub fn info(message: &str) {
    println!("{}[INFO]{} {}", Colours::BLUE, Colours::ENDC, message);
}

/// Prints a warning message to stdout.
pub fn warn(message: &str) {
    let sanitized = sanitize_error_message(message);
    println!("{}[WARN]{} {}", Colours::YELLOW, Colours::ENDC, sanitized);
}

/// Prints the RegicideOS ASCII banner.
pub fn print_banner() {
    println!("{}", Colours::BLUE);
    println!(
        r#"
    ██████╗ ███████╗ ██████╗ ██╗ ██████╗██╗██████╗ ███████╗ ██████╗ ███████╗
    ██╔══██╗██╔════╝██╔════╝ ██║██╔════╝██║██╔══██╗██╔════╝██╔═══██╗██╔════╝
    ██████╔╝█████╗  ██║  ███╗██║██║     ██║██║  ██║█████╗  ██║   ██║███████╗
    ██╔══██╗██╔══╝  ██║   ██║██║██║     ██║██║  ██║██╔══╝  ██║   ██║╚════██║
    ██║  ██║███████╗╚██████╔╝██║╚██████╗██║██████╔╝███████╗╚██████╔╝███████║
    ╚═╝  ╚═╝╚══════╝ ╚═════╝ ╚═╝ ╚═════╝╚═╝╚═════╝ ╚══════╝ ╚═════╝ ╚══════╝

              🏰 A Rust-first, AI-powered Linux Distribution 🚀
                        Cosmic Desktop • BTRFS • Gentoo Base
    "#
    );
    println!("{}", Colours::ENDC);
}
