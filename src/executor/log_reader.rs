//! Log file reading utilities
//!
//! Handles reading Stata log files, which can be:
//! - Large (MB or GB for long-running scripts)
//! - Still being written (need to read final state)
//! - Missing (if Stata crashed before writing)

use crate::error::Result;
use std::fs::File;
use std::io::{BufReader, Read as _, Seek, SeekFrom};
use std::path::Path;

/// Read the last N lines of a log file
///
/// This is more efficient than reading the entire file for large logs.
/// Error detection only needs the last 20-50 lines.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use stacy::executor::log_reader::read_last_lines;
///
/// let log = Path::new("script.log");
/// let lines = read_last_lines(log, 20)?;
///
/// for line in lines {
///     println!("{}", line);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn read_last_lines(log_file: &Path, n: usize) -> Result<Vec<String>> {
    let file = File::open(log_file)?;
    let mut reader = BufReader::new(file);

    // For small files, just read all lines
    let file_size = reader.seek(SeekFrom::End(0))?;
    if file_size < 10_000 {
        // File < 10KB, read everything
        reader.seek(SeekFrom::Start(0))?;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        let text = String::from_utf8_lossy(&buf);
        let lines: Vec<String> = text.lines().map(String::from).collect();
        return Ok(lines.into_iter().rev().take(n).rev().collect());
    }

    // For large files, read backwards from end
    // Strategy: Read last 5KB (typical for last 20-50 lines)
    let read_size = 5_000.min(file_size);
    reader.seek(SeekFrom::End(-(read_size as i64)))?;

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    let lines: Vec<String> = text.lines().map(String::from).collect();

    // Take last n lines
    Ok(lines.into_iter().rev().take(n).rev().collect())
}

/// Read the entire log file
///
/// Use sparingly - prefer read_last_lines() for error detection.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use stacy::executor::log_reader::read_full_log;
///
/// let log = Path::new("script.log");
/// let content = read_full_log(log)?;
///
/// println!("{}", content);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn read_full_log(log_file: &Path) -> Result<String> {
    let bytes = std::fs::read(log_file)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Count lines without loading the file into memory.
///
/// Matches `str::lines` semantics: a trailing line without a final newline
/// still counts.
fn count_lines(path: &Path) -> Result<usize> {
    use std::io::Read;
    let mut reader = BufReader::new(File::open(path)?);
    let mut buf = [0u8; 64 * 1024];
    let mut count = 0usize;
    let mut last_byte = b'\n';
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        count += buf[..n].iter().filter(|&&b| b == b'\n').count();
        last_byte = buf[n - 1];
    }
    if last_byte != b'\n' {
        count += 1;
    }
    Ok(count)
}

/// Get error context from log file (last 20 lines, formatted)
///
/// Used for default verbosity mode - shows context when error occurs.
///
/// Returns formatted string with actual line numbers from log file.
pub fn get_error_context(log_file: &Path) -> Result<String> {
    // Count lines in fixed-size chunks — long runs can produce logs too
    // large to load for numbering alone.
    let total_lines = count_lines(log_file)?;

    let last_lines = read_last_lines(log_file, 20)?;
    let start_idx = total_lines.saturating_sub(last_lines.len());

    let mut output = String::new();
    output.push('\n');
    output.push_str("─────────────────────────────────────────────────────────────\n");
    output.push_str("Last 20 lines of log:\n");
    output.push_str("─────────────────────────────────────────────────────────────\n");

    // Show actual line numbers from file
    for (i, line) in last_lines.iter().enumerate() {
        let line = line.as_str();
        let line_num = start_idx + i + 1; // +1 for 1-indexed

        // Highlight lines with r() codes
        if line.contains("r(") && line.contains(");") {
            output.push_str(&format!("{:3} → {}\n", line_num, line));
        } else {
            output.push_str(&format!("{:3} │ {}\n", line_num, line));
        }
    }

    output.push_str("─────────────────────────────────────────────────────────────\n");

    Ok(output)
}

/// What the streamer emits for each log line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamMode {
    /// Every line as Stata wrote it (`-v`, `-vv`)
    Raw,
    /// Boilerplate-stripped: no command echoes, blanks collapsed, output
    /// stops at the `end of do-file` trailer (TTY default)
    Clean,
}

/// Line filter implementing the Clean mode rules. Mirrors `strip_boilerplate`
/// but works incrementally on a live stream. Blank lines are held back until
/// the next content line — that both collapses runs of blanks and avoids
/// emitting a trailing blank before the `end of do-file` trailer (which a
/// stream, unlike post-hoc stripping, could not retract).
struct CleanFilter {
    echo: EchoFilter,
    seen_content: bool,
    pending_blank: bool,
    suppress: bool,
}

enum CleanAction {
    Skip,
    Emit,
    EmitWithLeadingBlank,
}

impl CleanFilter {
    fn new() -> Self {
        Self {
            echo: EchoFilter::new(),
            seen_content: false,
            pending_blank: false,
            suppress: false,
        }
    }

    fn process(&mut self, line: &str) -> CleanAction {
        if self.suppress {
            return CleanAction::Skip;
        }
        let trimmed = line.trim();
        if trimmed == "end of do-file" {
            // Trailer reached: suppress it and everything after (r(CODE);).
            // A script that *prints* this exact line will truncate the clean
            // stream — same tradeoff strip_boilerplate makes.
            self.suppress = true;
            return CleanAction::Skip;
        }
        // Echo detection must see every line, in order — it is stateful.
        if self.echo.is_echo(line) {
            return CleanAction::Skip;
        }
        if trimmed.is_empty() {
            // Hold back until we know content follows
            self.pending_blank = self.seen_content;
            return CleanAction::Skip;
        }
        self.seen_content = true;
        if self.pending_blank {
            self.pending_blank = false;
            CleanAction::EmitWithLeadingBlank
        } else {
            CleanAction::Emit
        }
    }
}

/// Stream a Stata log to stdout in real-time while the process runs.
///
/// Termination is driven by `stop`, which the caller sets once the Stata
/// process has exited (see `StataExecutor::run_internal`). This is the only
/// reliable signal: marker strings can be forged by script output, and a
/// killed Stata never writes one at all. After `stop` is observed, one final
/// drain pass picks up everything Stata flushed before exiting.
///
/// Robustness properties:
/// - Log never created (launch failure): returns once `stop` is set instead
///   of spinning forever
/// - Truncated/recreated log: position resets instead of seeking past EOF
/// - Partially written lines: held back until the newline arrives so Clean
///   filtering never sees fragments
/// - Closed stdout (e.g. piped to `head`): stops emitting, keeps draining,
///   returns cleanly instead of panicking
pub fn stream_log(
    log_file: &Path,
    poll_interval: std::time::Duration,
    mode: StreamMode,
    stop: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    let mut stdout = std::io::stdout();
    stream_log_to(log_file, poll_interval, mode, stop, &mut stdout)
}

/// Writer-generic core of [`stream_log`] (separated for testability).
pub fn stream_log_to(
    log_file: &Path,
    poll_interval: std::time::Duration,
    mode: StreamMode,
    stop: &std::sync::atomic::AtomicBool,
    out: &mut dyn std::io::Write,
) -> Result<()> {
    use std::io::{BufRead, BufReader, Seek, SeekFrom};
    use std::sync::atomic::Ordering;
    use std::thread::sleep;

    // Wait for the log to appear. If the process exits first, no log is
    // coming — the caller diagnoses the launch failure from captured stderr.
    while !log_file.exists() {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        sleep(poll_interval);
    }

    let mut reader = BufReader::new(File::open(log_file)?);
    let mut position = 0u64;
    let mut filter = CleanFilter::new();
    let mut writer_open = true;
    // Set when `stop` is observed: one more read pass to EOF, then done.
    let mut final_pass = false;

    loop {
        reader.seek(SeekFrom::Start(position))?;

        let mut buffer = String::new();
        let bytes_read = reader.read_line(&mut buffer)?;

        if bytes_read > 0 && (buffer.ends_with('\n') || final_pass) {
            position += bytes_read as u64;

            let action = match mode {
                StreamMode::Raw => CleanAction::Emit,
                StreamMode::Clean => filter.process(&buffer),
            };
            if writer_open {
                let write_result = match action {
                    CleanAction::Skip => Ok(()),
                    CleanAction::Emit => out.write_all(buffer.as_bytes()),
                    CleanAction::EmitWithLeadingBlank => out
                        .write_all(b"\n")
                        .and_then(|_| out.write_all(buffer.as_bytes())),
                };
                if write_result.is_err() {
                    // Downstream closed (broken pipe). Keep draining so we
                    // terminate normally, just stop emitting.
                    writer_open = false;
                }
            }
            continue;
        }

        // EOF, or a partial line still being written.
        if final_pass {
            break;
        }
        if stop.load(Ordering::Acquire) {
            // Process exited; drain whatever remains, then finish.
            final_pass = true;
            continue;
        }

        sleep(poll_interval);

        // Reopen to pick up truncation/recreation; reset position if the
        // file shrank so we don't seek past EOF and stall forever.
        match File::open(log_file) {
            Ok(f) => {
                if f.metadata().map(|m| m.len() < position).unwrap_or(false) {
                    position = 0;
                    filter = CleanFilter::new();
                }
                reader = BufReader::new(f);
            }
            Err(_) => break, // log deleted out from under us
        }
    }

    if writer_open {
        let _ = out.flush();
    }
    Ok(())
}

/// Width at which Stata wraps a log line (default `linesize`). A line this
/// long was cut off, so the `> ` line after it continues it.
const WRAP_WIDTH: usize = 79;

/// A multi-line construct whose body lines Stata echoes as `  2. body`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    /// `foreach`/`forvalues`/`while`/`if` — body ends when the braces balance
    Braces,
    /// `program [define] name` — body ends at `end`
    Program,
    /// `input` — the typed data lines end at `end`
    Input,
}

/// Decides, line by line, which log lines are Stata echoing back what it was
/// told to run — as opposed to results, which must survive.
///
/// Echoes cannot be recognised from a line's text alone, because results look
/// like them: `list` numbers its rows `  1. | ... |` exactly as Stata numbers
/// the body of a loop, `display "> x"` prints `> x`, and `display .` prints a
/// bare `.`. What separates the two is *where* they can occur:
///
/// - A command echo is a `. ` prompt in column 0. Nothing else starts one.
/// - A `> ` line continues the echo above it — but only if that echo was cut
///   off at [`WRAP_WIDTH`], or `#delimit ;` is in force (which echoes every
///   continuation line that way). Output wraps with `> ` too, so a `> ` line
///   after *output* is output.
/// - A numbered line is a body line of a block the previous echoes opened
///   (a loop, a program, `input`). Outside a block, `  1. | ... |` is a result.
///
/// Feed it every line of the log in order, including blank ones.
///
/// Known limit: a result line that itself starts in column 0 with `. `
/// (a value label such as `. missing` filling its column exactly) is
/// indistinguishable from an echo and is still dropped.
struct EchoFilter {
    /// Text of the command echo being read, minus its prompt. Continuation
    /// lines append to it; it is applied to the block state once complete.
    pending: String,
    /// Width of the previous line, if that line was part of an echo.
    prev_echo_width: Option<usize>,
    /// Block whose body Stata is currently echoing.
    block: Option<Block>,
    /// Open braces inside a [`Block::Braces`].
    depth: usize,
    /// `#delimit ;` in force: command echoes continue with `> ` at any width.
    delimit_semi: bool,
}

impl EchoFilter {
    fn new() -> Self {
        Self {
            pending: String::new(),
            prev_echo_width: None,
            block: None,
            depth: 0,
            delimit_semi: false,
        }
    }

    /// Is `line` (one log line, trailing newline optional) a command echo?
    fn is_echo(&mut self, line: &str) -> bool {
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        // Continuation of the echo we are already reading.
        if let Some(rest) = line.strip_prefix("> ") {
            if self.continues_echo() {
                self.pending.push_str(rest);
                self.prev_echo_width = Some(line.chars().count());
                return true;
            }
        }

        // Any other line completes the echo we were reading.
        self.finish_pending();

        // A command echo: `. ` prompt in column 0. A blank line in the script
        // echoes as `. ` (prompt, nothing after); a bare `.` with no trailing
        // space is a missing value that a command printed.
        if let Some(rest) = line.strip_prefix(". ") {
            self.pending = rest.to_string();
            self.prev_echo_width = Some(line.chars().count());
            return true;
        }

        // A numbered body line, but only while a block is open — Stata echoes
        // a whole block before running it, so nothing else can appear here.
        if self.block.is_some() {
            if let Some(rest) = numbered_body(line) {
                self.pending = rest.to_string();
                self.prev_echo_width = Some(line.chars().count());
                return true;
            }
        }

        self.prev_echo_width = None;

        // A line that is not an echo, inside what we read as a loop or program
        // body, means we misread the opening line (`local brace {`). Close the
        // block so one bad guess cannot swallow the rest of the log. `input`
        // is exempt: it prints a variable header above the data lines.
        if matches!(self.block, Some(Block::Braces) | Some(Block::Program)) {
            self.block = None;
            self.depth = 0;
        }

        false
    }

    /// Can a `> ` line continue the previous line?
    fn continues_echo(&self) -> bool {
        match self.prev_echo_width {
            Some(width) => self.delimit_semi || width >= WRAP_WIDTH,
            None => false,
        }
    }

    /// Apply the finished command echo to the block and `#delimit` state.
    fn finish_pending(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        let cmd = pending.trim();
        if cmd.is_empty() {
            return;
        }

        if let Some(rest) = cmd.strip_prefix("#delimit") {
            self.delimit_semi = rest.trim_start().starts_with(';');
            return;
        }

        match self.block {
            Some(Block::Braces) => {
                if cmd.starts_with('}') {
                    self.depth = self.depth.saturating_sub(1);
                }
                if cmd.ends_with('{') {
                    self.depth += 1;
                }
                if self.depth == 0 {
                    self.block = None;
                }
            }
            Some(Block::Program) | Some(Block::Input) => {
                if cmd == "end" {
                    self.block = None;
                }
            }
            None => {
                // Stata requires the opening brace to end the line.
                if cmd.ends_with('{') {
                    self.block = Some(Block::Braces);
                    self.depth = 1;
                } else if let Some(block) = opens_block(cmd) {
                    self.block = Some(block);
                }
            }
        }
    }
}

/// Does `cmd` open a body that Stata echoes and terminates with `end`?
///
/// Unrecognised spellings only mean the body is shown, never that a result is
/// dropped: a body line that arrives without an open block is treated as
/// output, and [`EchoFilter::is_echo`] closes a block it opened wrongly as
/// soon as output appears.
fn opens_block(cmd: &str) -> Option<Block> {
    // Prefix commands run the command that follows: `capture program drop x`
    // must not read as a program definition.
    let mut words = cmd.split_whitespace().skip_while(|w| {
        matches!(
            *w,
            "capture" | "cap" | "quietly" | "qui" | "noisily" | "noi"
        )
    });

    match words.next()? {
        "program" => match words.next().unwrap_or("define") {
            "drop" | "dir" | "list" => None,
            _ => Some(Block::Program),
        },
        "input" => Some(Block::Input),
        _ => None,
    }
}

/// Split a numbered body line (`  2. display x`) into its text, or `None`.
///
/// Stata right-aligns the number, then writes `. ` and the script line as
/// written — including the script's own indentation.
fn numbered_body(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches(' ');
    let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return None;
    }
    let after = &rest[digits..];
    // `N. body`, or `N.` alone (the script line was blank).
    match after.strip_prefix('.') {
        Some("") => Some(""),
        Some(body) => body.strip_prefix(' '),
        None => None,
    }
}

/// Strip Stata boilerplate from log output, returning only substantive content
///
/// Removes:
/// - Command echoes: the `. command` prompt, its `> ` continuations, and the
///   numbered body lines of loops, programs and `input` (see [`EchoFilter`])
/// - Leading blank lines
/// - `end of do-file` marker and everything after it (including `r(CODE);`)
/// - Collapses consecutive blank lines to a single blank line
/// - Trims trailing whitespace
///
/// Results are kept, including the ones that look like echoes: `list` rows,
/// the `.` row of `tabulate, missing`, and wrapped output continued with `> `.
pub fn strip_boilerplate(log_content: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut echo = EchoFilter::new();

    for line in log_content.lines() {
        // Stop at end-of-do-file marker
        if line.trim() == "end of do-file" {
            break;
        }

        // Skip command echo lines
        if echo.is_echo(line) {
            continue;
        }

        lines.push(line);
    }

    // Remove leading blank lines
    while lines.first().is_some_and(|l| l.trim().is_empty()) {
        lines.remove(0);
    }

    // Remove trailing blank lines
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }

    // Collapse consecutive blank lines
    let mut result = String::new();
    let mut prev_blank = false;
    for line in &lines {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line.trim_end());
        prev_blank = is_blank;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_last_lines_small_file() -> Result<()> {
        let mut temp = NamedTempFile::new()?;
        writeln!(temp, "line 1")?;
        writeln!(temp, "line 2")?;
        writeln!(temp, "line 3")?;
        writeln!(temp, "line 4")?;
        writeln!(temp, "line 5")?;
        temp.flush()?;

        let lines = read_last_lines(temp.path(), 3)?;
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line 3");
        assert_eq!(lines[1], "line 4");
        assert_eq!(lines[2], "line 5");

        Ok(())
    }

    // =========================================================================
    // strip_boilerplate tests
    // =========================================================================

    #[test]
    fn test_strip_boilerplate_typical_success() {
        let log = "\n\n\
. display 1+1\n\
2\n\
\n\
. display \"hello\"\n\
hello\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "2\n\nhello");
    }

    #[test]
    fn test_strip_boilerplate_error_log() {
        let log = "\n\
. invalid_command\n\
unrecognized command:  invalid_command\n\
r(199);\n\
\n\
end of do-file\n\
\n\
r(199);\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "unrecognized command:  invalid_command\nr(199);");
    }

    #[test]
    fn test_strip_boilerplate_empty_output() {
        let log = "\n\n\
. quietly display 1+1\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "");
    }

    #[test]
    fn test_strip_boilerplate_collapse_blank_lines() {
        let log = "\n\
. display 1\n\
1\n\
\n\
\n\
\n\
. display 2\n\
2\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "1\n\n2");
    }

    #[test]
    fn test_strip_boilerplate_no_end_marker() {
        // If log is truncated (no end-of-do-file), include everything
        let log = ". display 1\n1\n\n. display 2\n2\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "1\n\n2");
    }

    #[test]
    fn test_strip_boilerplate_bare_dot_continuation() {
        let log = "\n\
. foreach x of numlist 1/3 {\n\
.   display `x'\n\
. }\n\
1\n\
2\n\
3\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "1\n2\n3");
    }

    #[test]
    fn test_strip_boilerplate_numbered_continuation() {
        // Stata shows numbered lines inside loops in interactive-style logs
        let log = "\n\
. foreach x of numlist 1/3 {\n\
  2.   display `x'\n\
  3. }\n\
1\n\
2\n\
3\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "1\n2\n3");
    }

    #[test]
    fn test_strip_boilerplate_program_define() {
        let log = "\n\
. program define myprog\n\
  2.   display \"hello from myprog\"\n\
  3. end\n\
\n\
. myprog\n\
hello from myprog\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "hello from myprog");
    }

    #[test]
    fn test_strip_boilerplate_high_numbered_lines() {
        // Double-digit line numbers inside a long block
        let log = "\n\
. program define longprog\n\
  2.   local a = 1\n\
  3.   local b = 2\n\
  10.   display `a' + `b'\n\
  11. end\n\
\n\
. longprog\n\
3\n\
\n\
end of do-file\n";

        let result = strip_boilerplate(log);
        assert_eq!(result, "3");
    }

    // =========================================================================
    // EchoFilter tests
    //
    // The log excerpts below are copied from real StataNow 19 batch logs.
    // =========================================================================

    /// Which lines of `log` the filter calls echoes.
    fn echoes(log: &str) -> Vec<&str> {
        let mut filter = EchoFilter::new();
        log.lines()
            .filter(|line| filter.is_echo(line))
            .collect::<Vec<_>>()
    }

    /// Which lines of `log` the filter keeps.
    fn kept(log: &str) -> Vec<&str> {
        let mut filter = EchoFilter::new();
        log.lines()
            .filter(|line| !filter.is_echo(line))
            .collect::<Vec<_>>()
    }

    /// Pad `prefix` out to the width at which Stata wraps a log line.
    fn at_wrap_width(prefix: &str) -> String {
        let mut line = prefix.to_string();
        while line.chars().count() < WRAP_WIDTH {
            line.push('x');
        }
        line
    }

    #[test]
    fn test_echo_at_column_zero_is_stripped() {
        // The last echo is the `. ` prompt a blank script line produces.
        let log = ". sysuse auto, clear\n(1978 automobile data)\n. display 1+1\n2\n. \n";
        assert_eq!(kept(log), vec!["(1978 automobile data)", "2"]);
    }

    #[test]
    fn test_keeps_list_rows() {
        // Every data row of `list` is numbered exactly like a loop body.
        let log = "\
. list make price mpg in 1/3

     +-----------------------------+
     | make            price   mpg |
     |-----------------------------|
  1. | AMC Concord     4,099    22 |
  2. | AMC Pacer       4,749    17 |
  3. | AMC Spirit      3,799    22 |
     +-----------------------------+
";
        assert_eq!(echoes(log), vec![". list make price mpg in 1/3"]);
        assert!(kept(log).contains(&"  1. | AMC Concord     4,099    22 |"));
        assert!(kept(log).contains(&"  3. | AMC Spirit      3,799    22 |"));
    }

    #[test]
    fn test_keeps_borderless_list_rows() {
        // `list, clean noheader` puts the first row directly under the echo,
        // and its rows carry no `|` to key off.
        let log = "\
. list make mpg in 1/2, clean noheader
  1.   AMC Concord    22
  2.   AMC Pacer      17
";
        assert_eq!(echoes(log), vec![". list make mpg in 1/2, clean noheader"]);
        assert_eq!(
            kept(log),
            vec!["  1.   AMC Concord    22", "  2.   AMC Pacer      17"]
        );
    }

    #[test]
    fn test_keeps_tabulate_missing_row() {
        // The `.` category must survive, or the rows stop summing to Total.
        let log = "\
. tabulate rep78, missing

     Repair |
record 1978 |      Freq.     Percent        Cum.
------------+-----------------------------------
          5 |         11       14.86       93.24
          . |          5        6.76      100.00
------------+-----------------------------------
      Total |         74      100.00
";
        assert!(kept(log).contains(&"          . |          5        6.76      100.00"));
    }

    #[test]
    fn test_keeps_echo_shaped_display_output() {
        let log = "\
. display \"1. step\"
1. step
. display \"> x\"
> x
. display .
.
";
        assert_eq!(kept(log), vec!["1. step", "> x", "."]);
    }

    #[test]
    fn test_keeps_wrapped_output() {
        // Output wraps with the same `> ` marker as a wrapped echo; the marker
        // continues whatever came before it.
        let output = at_wrap_width("a very long line of output that runs past ");
        let log = format!(". display \"...\"\n{}\n> and its tail\n", output);

        assert_eq!(kept(&log), vec![output.as_str(), "> and its tail"]);
    }

    #[test]
    fn test_strips_wrapped_command_echo() {
        let echo = at_wrap_width(". regress price mpg weight foreign headroom ");
        let log = format!("{}\n> _ratio\n\n      Source |       SS\n", echo);

        assert_eq!(echoes(&log), vec![echo.as_str(), "> _ratio"]);
    }

    #[test]
    fn test_strips_delimit_semicolon_continuation() {
        // Under `#delimit ;` every continuation line is echoed with `> `,
        // whatever its width.
        let log = "\
. #delimit ;
delimiter now ;
. display
>    \"delimit continuation\"
>    ;
delimit continuation
. #delimit cr
delimiter now cr
. display \"> x\"
> x
";
        assert_eq!(
            kept(log),
            vec![
                "delimiter now ;",
                "delimit continuation",
                "delimiter now cr",
                "> x",
            ]
        );
    }

    #[test]
    fn test_strips_loop_body() {
        let log = "\
. forvalues i = 1/2 {
  2.     display `i'
  3. }
1
2
";
        assert_eq!(kept(log), vec!["1", "2"]);
    }

    #[test]
    fn test_strips_nested_loop_body() {
        let log = "\
. forvalues i = 1/2 {
  2.     foreach v of varlist price mpg {
  3.         display \"`i' `v'\"
  4.     }
  5. }
1 price
";
        assert_eq!(kept(log), vec!["1 price"]);
    }

    #[test]
    fn test_strips_program_body() {
        let log = "\
. program define myprog
  1.     display \"in prog\"
  2. end

. myprog
in prog
";
        assert_eq!(kept(log), vec!["", "in prog"]);
    }

    #[test]
    fn test_program_drop_does_not_open_a_block() {
        let log = "\
. capture program drop myprog
. list make in 1/1
     +-------------+
  1. | AMC Concord |
     +-------------+
";
        assert!(kept(log).contains(&"  1. | AMC Concord |"));
    }

    #[test]
    fn test_strips_input_block() {
        // `input` echoes the typed data lines, but prints a header between the
        // command and the first of them.
        let log = "\
. input x y

             x          y
  1. 1 2
  2. 3 4
  3. end
";
        assert_eq!(kept(log), vec!["", "             x          y"]);
    }

    #[test]
    fn test_misread_block_opener_recovers() {
        // `local brace {` ends in a brace but opens nothing. The rows below
        // must not be swallowed by a block that never existed.
        let log = "\
. local brace {
. list make in 1/2

     +-------------+
  1. | AMC Concord |
  2. | AMC Pacer   |
     +-------------+
";
        assert!(kept(log).contains(&"  1. | AMC Concord |"));
        assert!(kept(log).contains(&"  2. | AMC Pacer   |"));
    }

    #[test]
    fn test_strip_boilerplate_keeps_list_rows() {
        let log = concat!(
            ". sysuse auto, clear\n",
            "(1978 automobile data)\n",
            "\n",
            ". list make mpg in 1/2\n",
            "\n",
            "     +-------------------+\n",
            "     | make          mpg |\n",
            "     |-------------------|\n",
            "  1. | AMC Concord    22 |\n",
            "  2. | AMC Pacer      17 |\n",
            "     +-------------------+\n",
            "\n",
            ". \n",
            "end of do-file\n",
        );
        assert_eq!(
            strip_boilerplate(log),
            concat!(
                "(1978 automobile data)\n",
                "\n",
                "     +-------------------+\n",
                "     | make          mpg |\n",
                "     |-------------------|\n",
                "  1. | AMC Concord    22 |\n",
                "  2. | AMC Pacer      17 |\n",
                "     +-------------------+",
            )
        );
    }

    // =========================================================================
    // stream_log_to tests
    // =========================================================================

    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    const POLL: Duration = Duration::from_millis(5);

    /// Spawn the streamer against `path`; returns a handle yielding captured output.
    fn stream_in_thread(
        path: std::path::PathBuf,
        mode: StreamMode,
        stop: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            stream_log_to(&path, POLL, mode, &stop, &mut buf).unwrap();
            buf
        })
    }

    #[test]
    fn test_stream_raw_incremental_writes() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("run.log");
        std::fs::write(&log, "").unwrap();

        let stop = Arc::new(AtomicBool::new(false));
        let handle = stream_in_thread(log.clone(), StreamMode::Raw, stop.clone());

        let mut f = std::fs::OpenOptions::new().append(true).open(&log).unwrap();
        writeln!(f, ". display 1").unwrap();
        writeln!(f, "1").unwrap();
        f.flush().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        writeln!(f, "end of do-file").unwrap();
        writeln!(f, "r(199);").unwrap();
        f.flush().unwrap();

        stop.store(true, Ordering::Release);
        let out = String::from_utf8(handle.join().unwrap()).unwrap();
        // Raw mode: everything, including echo, marker, and trailer
        assert_eq!(out, ". display 1\n1\nend of do-file\nr(199);\n");
    }

    #[test]
    fn test_stream_clean_filters_echo_and_trailer() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("run.log");
        std::fs::write(
            &log,
            "\n. display 1\n1\n\n\n. display 2\n2\n\nend of do-file\n\nr(199);\n",
        )
        .unwrap();

        // Stop pre-set: streamer drains the complete file and exits.
        let stop = Arc::new(AtomicBool::new(true));
        let handle = stream_in_thread(log, StreamMode::Clean, stop);

        let out = String::from_utf8(handle.join().unwrap()).unwrap();
        assert_eq!(out, "1\n\n2\n");
    }

    #[test]
    fn test_stream_clean_keeps_result_rows() {
        // The streamed table must carry its data rows, not just its frame.
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("run.log");
        std::fs::write(
            &log,
            concat!(
                ". list make mpg in 1/2\n",
                "\n",
                "     +-------------------+\n",
                "     | make          mpg |\n",
                "     |-------------------|\n",
                "  1. | AMC Concord    22 |\n",
                "  2. | AMC Pacer      17 |\n",
                "     +-------------------+\n",
                "\n",
                ". \n",
                "end of do-file\n",
                "\n",
                "r(0);\n",
            ),
        )
        .unwrap();

        let stop = Arc::new(AtomicBool::new(true));
        let handle = stream_in_thread(log, StreamMode::Clean, stop);

        let out = String::from_utf8(handle.join().unwrap()).unwrap();
        assert_eq!(
            out,
            concat!(
                "     +-------------------+\n",
                "     | make          mpg |\n",
                "     |-------------------|\n",
                "  1. | AMC Concord    22 |\n",
                "  2. | AMC Pacer      17 |\n",
                "     +-------------------+\n",
            )
        );
    }

    #[test]
    fn test_stream_terminates_when_log_never_created() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("never.log");

        let stop = Arc::new(AtomicBool::new(false));
        let handle = stream_in_thread(log, StreamMode::Raw, stop.clone());

        std::thread::sleep(Duration::from_millis(30));
        stop.store(true, Ordering::Release);
        // Must terminate (previously spun forever) with no output.
        let out = handle.join().unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn test_stream_terminates_without_end_marker() {
        // Killed Stata: log exists but no trailer was ever written.
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("killed.log");
        std::fs::write(&log, ". sleep 100000\n").unwrap();

        let stop = Arc::new(AtomicBool::new(false));
        let handle = stream_in_thread(log, StreamMode::Raw, stop.clone());

        std::thread::sleep(Duration::from_millis(30));
        stop.store(true, Ordering::Release);
        let out = String::from_utf8(handle.join().unwrap()).unwrap();
        assert_eq!(out, ". sleep 100000\n");
    }

    #[test]
    fn test_stream_recovers_from_truncation() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("trunc.log");
        std::fs::write(&log, "first phase line\n").unwrap();

        let stop = Arc::new(AtomicBool::new(false));
        let handle = stream_in_thread(log.clone(), StreamMode::Raw, stop.clone());

        // Let the streamer read past what the truncated file will hold
        std::thread::sleep(Duration::from_millis(50));
        std::fs::write(&log, "new\n").unwrap(); // truncate + rewrite, shorter
        std::thread::sleep(Duration::from_millis(50));

        stop.store(true, Ordering::Release);
        let out = String::from_utf8(handle.join().unwrap()).unwrap();
        // Old content was already streamed; new content must appear too
        // (previously: seek past EOF and stall forever).
        assert_eq!(out, "first phase line\nnew\n");
    }

    #[test]
    fn test_stream_holds_back_partial_lines() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("partial.log");
        std::fs::write(&log, "hello").unwrap(); // no newline yet

        let stop = Arc::new(AtomicBool::new(false));
        let handle = stream_in_thread(log.clone(), StreamMode::Raw, stop.clone());

        std::thread::sleep(Duration::from_millis(50));
        let mut f = std::fs::OpenOptions::new().append(true).open(&log).unwrap();
        writeln!(f, " world").unwrap();
        f.flush().unwrap();
        std::thread::sleep(Duration::from_millis(50));

        stop.store(true, Ordering::Release);
        let out = String::from_utf8(handle.join().unwrap()).unwrap();
        assert_eq!(out, "hello world\n");
    }

    #[test]
    fn test_stream_survives_closed_writer() {
        struct BrokenPipe;
        impl Write for BrokenPipe {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
            }
        }

        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("pipe.log");
        std::fs::write(&log, "line 1\nline 2\nend of do-file\n").unwrap();

        let stop = AtomicBool::new(true);
        let mut out = BrokenPipe;
        // Must return Ok, not Err or panic, when downstream is closed.
        stream_log_to(&log, POLL, StreamMode::Raw, &stop, &mut out).unwrap();
    }

    #[test]
    fn test_stream_clean_ignores_marker_lookalike_in_raw_mode() {
        // Raw mode must not terminate early on marker-lookalike output;
        // everything after it still streams.
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("fake.log");
        std::fs::write(&log, "end of do-file\nmore output after\n").unwrap();

        let stop = Arc::new(AtomicBool::new(true));
        let handle = stream_in_thread(log, StreamMode::Raw, stop);
        let out = String::from_utf8(handle.join().unwrap()).unwrap();
        assert_eq!(out, "end of do-file\nmore output after\n");
    }

    #[test]
    fn test_read_full_log_with_non_utf8() -> Result<()> {
        let mut temp = NamedTempFile::new()?;
        // Latin-1 "résultat" (é = 0xe9, è = 0xe8)
        temp.write_all(b"variable label: r\xe9sultat du mod\xe8le\n")?;
        temp.write_all(b"end of do-file\n")?;
        temp.flush()?;

        let content = read_full_log(temp.path())?;
        assert!(content.contains("end of do-file"));
        assert!(content.contains("variable label:"));
        Ok(())
    }

    #[test]
    fn test_count_lines() -> Result<()> {
        let mut temp = NamedTempFile::new()?;
        write!(temp, "a\nb\nc\n")?;
        temp.flush()?;
        assert_eq!(count_lines(temp.path())?, 3);

        // No trailing newline: partial last line still counts
        let mut temp2 = NamedTempFile::new()?;
        write!(temp2, "a\nb\nc")?;
        temp2.flush()?;
        assert_eq!(count_lines(temp2.path())?, 3);

        let temp3 = NamedTempFile::new()?;
        assert_eq!(count_lines(temp3.path())?, 0);
        Ok(())
    }

    #[test]
    fn test_get_error_context_line_numbers() -> Result<()> {
        let mut temp = NamedTempFile::new()?;
        for i in 1..=30 {
            writeln!(temp, "line number {}", i)?;
        }
        writeln!(temp, "r(601);")?;
        temp.flush()?;

        let context = get_error_context(temp.path())?;
        // 31 lines total; window is the last 20 → lines 12..=31
        assert!(context.contains(" 12 │ line number 12"));
        assert!(context.contains(" 31 → r(601);"));
        assert!(!context.contains("line number 11\n"));
        Ok(())
    }

    #[test]
    fn test_get_error_context_with_non_utf8() -> Result<()> {
        let mut temp = NamedTempFile::new()?;
        temp.write_all(b"label: r\xe9sultat\n")?;
        temp.write_all(b"file data.dta not found\n")?;
        temp.write_all(b"r(601);\n")?;
        temp.flush()?;

        let context = get_error_context(temp.path())?;
        assert!(context.contains("r(601)"));
        Ok(())
    }
}
