/// Verbosity levels for execution output
///
/// Controls how much output is shown during Stata execution.
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Verbosity {
    /// Suppress all output, even error context (for CI/batch)
    ///
    /// ```bash
    /// stacy run script.do --quiet
    /// ```
    Quiet = 0,

    /// Non-TTY default: quiet during execution, show clean output post-hoc
    ///
    /// Used when stdout is piped (not a terminal).
    ///
    /// ```bash
    /// stacy run script.do | less
    /// ```
    #[default]
    PipedDefault = 1,

    /// TTY default: "Running..." feedback + stream clean output in real-time
    ///
    /// Used when stdout is a terminal (interactive use).
    ///
    /// ```bash
    /// stacy run script.do
    /// ```
    DefaultInteractive = 2,

    /// Verbose: stream raw log output in real-time
    ///
    /// Like `tail -f` on the log file while Stata runs.
    ///
    /// ```bash
    /// stacy run script.do -v
    /// ```
    Verbose = 3,

    /// Very verbose: show execution details + stream raw output
    ///
    /// Shows:
    /// - Stata binary being used
    /// - S_ADO environment variable
    /// - Full command being run
    /// - Log file location
    /// - Then streams output like `-v`
    ///
    /// ```bash
    /// stacy run script.do -vv
    /// ```
    VeryVerbose = 4,
}

impl Verbosity {
    /// Should we stream raw log output in real-time? (-v, -vv)
    pub fn should_stream_raw(&self) -> bool {
        matches!(self, Verbosity::Verbose | Verbosity::VeryVerbose)
    }

    /// Should we stream clean (boilerplate-stripped) output in real-time? (TTY default)
    pub fn should_stream_clean(&self) -> bool {
        matches!(self, Verbosity::DefaultInteractive)
    }

    /// Should we show error context (last 20 lines) on failure?
    ///
    /// Disabled: the CLI layer now handles all error display uniformly
    /// (FAIL → Error → See → Log context) for all verbosity modes.
    pub fn should_show_error_context(&self) -> bool {
        false
    }

    /// Should we show execution details (binary, command, etc.)?
    pub fn should_show_execution_details(&self) -> bool {
        matches!(self, Verbosity::VeryVerbose)
    }

    /// Should we show clean post-processed output after execution? (piped default only)
    ///
    /// Only at PipedDefault: Quiet suppresses everything, DefaultInteractive streams
    /// clean output in real-time, Verbose/VeryVerbose stream the raw log instead.
    pub fn should_show_clean_output_post_hoc(&self) -> bool {
        matches!(self, Verbosity::PipedDefault)
    }

    /// Should we show "Running..." indicator and PASS/FAIL status?
    pub fn should_show_running_indicator(&self) -> bool {
        matches!(
            self,
            Verbosity::DefaultInteractive | Verbosity::Verbose | Verbosity::VeryVerbose
        )
    }

    /// Is any output suppressed?
    pub fn is_quiet(&self) -> bool {
        matches!(self, Verbosity::Quiet)
    }
}

impl fmt::Display for Verbosity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verbosity::Quiet => write!(f, "quiet"),
            Verbosity::PipedDefault => write!(f, "default"),
            Verbosity::DefaultInteractive => write!(f, "interactive"),
            Verbosity::Verbose => write!(f, "verbose"),
            Verbosity::VeryVerbose => write!(f, "very-verbose"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_ordering() {
        assert!(Verbosity::Quiet < Verbosity::PipedDefault);
        assert!(Verbosity::PipedDefault < Verbosity::DefaultInteractive);
        assert!(Verbosity::DefaultInteractive < Verbosity::Verbose);
        assert!(Verbosity::Verbose < Verbosity::VeryVerbose);
    }

    #[test]
    fn test_should_stream_raw() {
        assert!(!Verbosity::Quiet.should_stream_raw());
        assert!(!Verbosity::PipedDefault.should_stream_raw());
        assert!(!Verbosity::DefaultInteractive.should_stream_raw());
        assert!(Verbosity::Verbose.should_stream_raw());
        assert!(Verbosity::VeryVerbose.should_stream_raw());
    }

    #[test]
    fn test_should_stream_clean() {
        assert!(!Verbosity::Quiet.should_stream_clean());
        assert!(!Verbosity::PipedDefault.should_stream_clean());
        assert!(Verbosity::DefaultInteractive.should_stream_clean());
        assert!(!Verbosity::Verbose.should_stream_clean());
        assert!(!Verbosity::VeryVerbose.should_stream_clean());
    }

    #[test]
    fn test_should_show_error_context() {
        // Disabled: CLI layer handles error display for all modes
        assert!(!Verbosity::Quiet.should_show_error_context());
        assert!(!Verbosity::PipedDefault.should_show_error_context());
        assert!(!Verbosity::DefaultInteractive.should_show_error_context());
        assert!(!Verbosity::Verbose.should_show_error_context());
        assert!(!Verbosity::VeryVerbose.should_show_error_context());
    }

    #[test]
    fn test_should_show_execution_details() {
        assert!(!Verbosity::Quiet.should_show_execution_details());
        assert!(!Verbosity::PipedDefault.should_show_execution_details());
        assert!(!Verbosity::DefaultInteractive.should_show_execution_details());
        assert!(!Verbosity::Verbose.should_show_execution_details());
        assert!(Verbosity::VeryVerbose.should_show_execution_details());
    }

    #[test]
    fn test_should_show_clean_output_post_hoc() {
        assert!(!Verbosity::Quiet.should_show_clean_output_post_hoc());
        assert!(Verbosity::PipedDefault.should_show_clean_output_post_hoc());
        assert!(!Verbosity::DefaultInteractive.should_show_clean_output_post_hoc());
        assert!(!Verbosity::Verbose.should_show_clean_output_post_hoc());
        assert!(!Verbosity::VeryVerbose.should_show_clean_output_post_hoc());
    }

    #[test]
    fn test_should_show_running_indicator() {
        assert!(!Verbosity::Quiet.should_show_running_indicator());
        assert!(!Verbosity::PipedDefault.should_show_running_indicator());
        assert!(Verbosity::DefaultInteractive.should_show_running_indicator());
        assert!(Verbosity::Verbose.should_show_running_indicator());
        assert!(Verbosity::VeryVerbose.should_show_running_indicator());
    }

    #[test]
    fn test_default() {
        assert_eq!(Verbosity::default(), Verbosity::PipedDefault);
    }
}
