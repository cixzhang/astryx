//! Hands links to the platform's default handler.

use std::io;
use std::process::{Command, Stdio};

/// Schemes passed on to the platform. Others could start programs or open
/// local files, so they are refused.
const SCHEMES: [&str; 3] = ["https", "http", "mailto"];

pub fn is_openable(url: &str) -> bool {
    let Some((scheme, rest)) = url.split_once(':') else {
        return false;
    };
    SCHEMES.contains(&scheme.to_ascii_lowercase().as_str())
        && !rest.is_empty()
        && !url.chars().any(char::is_control)
}

/// The process that opens `url`, or `None` when the URL is refused.
pub fn open_command(url: &str) -> Option<Command> {
    if !is_openable(url) {
        return None;
    }
    #[cfg(target_os = "macos")]
    let command = {
        let mut c = Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(windows)]
    let command = {
        // Opens the URL with its registered handler without a shell parsing it.
        let mut c = Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let command = {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    #[cfg(not(any(unix, windows)))]
    let command = return None;
    Some(command)
}

/// Opens `url` without waiting for the handler.
pub fn open_url(url: &str) -> io::Result<()> {
    let mut command = open_command(url).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("refused to open {url:?}"),
        )
    })?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // Reaps the handler process when it exits.
    std::thread::spawn(move || child.wait());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_and_mail_links_open() {
        assert!(is_openable("https://example.com/a?b=c"));
        assert!(is_openable("HTTP://example.com"));
        assert!(is_openable("mailto:someone@example.com"));
        assert!(!is_openable("file:///etc/passwd"));
        assert!(!is_openable("javascript:alert(1)"));
        assert!(!is_openable("/usr/bin/xterm"));
        assert!(!is_openable("https:"));
        assert!(!is_openable("https://example.com/\nrm"));
        assert!(open_command("ssh://host").is_none());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_uses_xdg_open_with_the_url_as_one_argument() {
        let c = open_command("https://example.com/a b").unwrap();
        assert_eq!(c.get_program(), "xdg-open");
        assert_eq!(
            c.get_args().collect::<Vec<_>>(),
            vec!["https://example.com/a b"]
        );
    }
}
