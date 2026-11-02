//! Opens a new window of the terminal we're running in.

use std::{
    io,
    process::{Command, Stdio},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Launcher {
    AppleTerminal,
    ITerm,
    Kitty,
    WezTerm,
    Alacritty,
    Ghostty,
    GnomeTerminal,
    Konsole,
}

impl Launcher {
    /// Detect the terminal we're running in using environment variables.
    pub(crate) fn detect(var: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let is_set = |name: &str| var(name).is_some_and(|v| !v.is_empty());
        let term_program = var("TERM_PROGRAM").unwrap_or_default();
        match term_program.as_str() {
            "Apple_Terminal" => return Some(Self::AppleTerminal),
            "iTerm.app" => return Some(Self::ITerm),
            "WezTerm" => return Some(Self::WezTerm),
            "ghostty" => return Some(Self::Ghostty),
            "kitty" => return Some(Self::Kitty),
            _ => (),
        };
        // These are preserved inside multiplexers like tmux, where TERM_PROGRAM is overridden.
        if is_set("KITTY_WINDOW_ID") {
            Some(Self::Kitty)
        } else if is_set("WEZTERM_PANE") {
            Some(Self::WezTerm)
        } else if is_set("GHOSTTY_RESOURCES_DIR") {
            Some(Self::Ghostty)
        } else if is_set("ALACRITTY_WINDOW_ID") || is_set("ALACRITTY_SOCKET") || is_set("ALACRITTY_LOG") {
            Some(Self::Alacritty)
        } else if is_set("KONSOLE_VERSION") {
            Some(Self::Konsole)
        } else if is_set("GNOME_TERMINAL_SCREEN") || is_set("GNOME_TERMINAL_SERVICE") {
            Some(Self::GnomeTerminal)
        } else {
            None
        }
    }

    /// The commands to try, in order, to open a new window running `args`.
    fn candidates(&self, args: &[String]) -> Vec<Candidate> {
        let with = |prefix: &[&str]| -> Vec<String> {
            prefix.iter().map(|s| s.to_string()).chain(args.iter().cloned()).collect()
        };
        match self {
            Self::AppleTerminal => {
                let script = format!(
                    "tell application \"Terminal\" to do script \"{}\"",
                    apple_script_escape(&shell_join(args))
                );
                vec![Candidate::checked("osascript", vec!["-e".into(), script])]
            }
            Self::ITerm => {
                let script = format!(
                    "tell application \"iTerm\" to create window with default profile command \"{}\"",
                    apple_script_escape(&shell_join(args))
                );
                vec![Candidate::checked("osascript", vec!["-e".into(), script])]
            }
            Self::Kitty => vec![
                Candidate::checked("kitty", with(&["@", "launch", "--type=os-window"])),
                Candidate::detached("kitty", args.to_vec()),
            ],
            Self::WezTerm => vec![
                Candidate::checked("wezterm", with(&["cli", "spawn", "--new-window", "--"])),
                Candidate::detached("wezterm", with(&["start", "--"])),
            ],
            Self::Alacritty => vec![Candidate::detached("alacritty", with(&["-e"]))],
            Self::Ghostty if cfg!(target_os = "macos") => {
                vec![Candidate::checked("open", with(&["-n", "-a", "Ghostty", "--args", "-e"]))]
            }
            Self::Ghostty => vec![Candidate::detached("ghostty", with(&["-e"]))],
            Self::GnomeTerminal => vec![Candidate::detached("gnome-terminal", with(&["--"]))],
            Self::Konsole => vec![Candidate::detached("konsole", with(&["-e"]))],
        }
    }

    /// Open a new window running the given command.
    pub(crate) fn launch(&self, args: &[String]) -> io::Result<()> {
        let mut last_error = io::Error::other("no launch command available");
        for candidate in self.candidates(args) {
            match candidate.run() {
                Ok(()) => return Ok(()),
                Err(e) => last_error = e,
            }
        }
        Err(last_error)
    }
}

#[derive(Debug, PartialEq)]
struct Candidate {
    program: &'static str,
    args: Vec<String>,
    // Whether to wait for the command to finish and require it to succeed.
    check_status: bool,
}

impl Candidate {
    fn checked(program: &'static str, args: Vec<String>) -> Self {
        Self { program, args, check_status: true }
    }

    fn detached(program: &'static str, args: Vec<String>) -> Self {
        Self { program, args, check_status: false }
    }

    fn run(&self) -> io::Result<()> {
        let mut command = Command::new(self.program);
        command.args(&self.args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if self.check_status {
            let status = command.status()?;
            if status.success() { Ok(()) } else { Err(io::Error::other(format!("{} failed: {status}", self.program))) }
        } else {
            command.spawn()?;
            Ok(())
        }
    }
}

/// Run a user provided command template, replacing `{cmd}` with the command to run.
/// Close the window this slave is running in, for terminals that leave it open.
///
/// Ghostty on macOS is launched as a dedicated instance that keeps running after its command exits.
/// That instance is found by looking for the ancestor process that was started with our slave flag.
pub(crate) fn close_slave_window() {
    if !cfg!(target_os = "macos") {
        return;
    }
    let mut pid = std::process::id();
    for _ in 0..6 {
        let Ok(output) =
            std::process::Command::new("ps").args(["-o", "ppid=,command=", "-p", &pid.to_string()]).output()
        else {
            return;
        };
        let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let Some((parent, _)) = line.split_once(char::is_whitespace) else {
            return;
        };
        let Ok(parent) = parent.trim().parse::<u32>() else {
            return;
        };
        if parent <= 1 {
            return;
        }
        let Ok(output) = std::process::Command::new("ps").args(["-o", "command=", "-p", &parent.to_string()]).output()
        else {
            return;
        };
        let command = String::from_utf8_lossy(&output.stdout);
        if command.contains("Ghostty.app") && command.contains("--dual-slave") {
            let _ = std::process::Command::new("kill").arg(parent.to_string()).status();
            return;
        }
        pid = parent;
    }
}

pub(crate) fn launch_from_template(template: &str, args: &[String]) -> io::Result<()> {
    let command = template.replace("{cmd}", &shell_join(args));
    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() { Ok(()) } else { Err(io::Error::other(format!("spawn command failed: {status}"))) }
}

pub(crate) fn shell_quote(arg: &str) -> String {
    let is_safe = !arg.is_empty()
        && arg.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '=' | ',' | '+'));
    if is_safe { arg.to_string() } else { format!("'{}'", arg.replace('\'', "'\\''")) }
}

pub(crate) fn shell_join(args: &[String]) -> String {
    args.iter().map(|a| shell_quote(a)).collect::<Vec<_>>().join(" ")
}

fn apple_script_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::collections::HashMap;

    fn detect(vars: &[(&str, &str)]) -> Option<Launcher> {
        let vars: HashMap<_, _> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Launcher::detect(|name| vars.get(name).cloned())
    }

    #[rstest]
    #[case::apple(&[("TERM_PROGRAM", "Apple_Terminal")], Some(Launcher::AppleTerminal))]
    #[case::iterm(&[("TERM_PROGRAM", "iTerm.app")], Some(Launcher::ITerm))]
    #[case::wezterm(&[("TERM_PROGRAM", "WezTerm")], Some(Launcher::WezTerm))]
    #[case::kitty(&[("KITTY_WINDOW_ID", "1")], Some(Launcher::Kitty))]
    #[case::kitty_in_tmux(&[("TERM_PROGRAM", "tmux"), ("KITTY_WINDOW_ID", "1")], Some(Launcher::Kitty))]
    #[case::alacritty(&[("ALACRITTY_SOCKET", "/tmp/x")], Some(Launcher::Alacritty))]
    #[case::konsole(&[("KONSOLE_VERSION", "1")], Some(Launcher::Konsole))]
    #[case::gnome(&[("GNOME_TERMINAL_SCREEN", "/x")], Some(Launcher::GnomeTerminal))]
    #[case::unknown(&[("TERM_PROGRAM", "vscode")], None)]
    #[case::empty(&[], None)]
    fn detection(#[case] vars: &[(&str, &str)], #[case] expected: Option<Launcher>) {
        assert_eq!(detect(vars), expected);
    }

    #[test]
    fn quoting() {
        assert_eq!(shell_quote("abc/def-1.md"), "abc/def-1.md");
        assert_eq!(shell_quote("my deck.md"), "'my deck.md'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn apple_terminal_script() {
        let args = vec!["/bin/presenterm".to_string(), "/tmp/my \"deck\".md".to_string()];
        let candidates = Launcher::AppleTerminal.candidates(&args);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].program, "osascript");
        let script = &candidates[0].args[1];
        assert!(script.contains("do script"));
        assert!(script.contains("'/tmp/my \\\"deck\\\".md'"), "{script}");
    }

    #[test]
    fn kitty_falls_back_to_new_process() {
        let args = vec!["presenterm".to_string(), "deck.md".to_string()];
        let candidates = Launcher::Kitty.candidates(&args);
        assert_eq!(candidates[0].args[..3], ["@", "launch", "--type=os-window"]);
        assert_eq!(candidates[1], Candidate::detached("kitty", args));
    }
}
