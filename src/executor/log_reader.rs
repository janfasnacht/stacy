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
        if is_command_echo(trimmed) {
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

/// Check if a trimmed line is a Stata command echo
///
/// Matches:
/// - `. command` — standard command echo
/// - `.` — bare continuation dot
/// - `  2. command` — numbered continuation inside loops/programs
/// - `> continuation` — long command wrapping or `#delimit ;` continuation
pub fn is_command_echo(trimmed: &str) -> bool {
    // Standard: `. ` prefix or bare `.`
    if trimmed.starts_with(". ") || trimmed == "." {
        return true;
    }

    // Continuation lines: `> ` prefix from long commands or #delimit ; mode
    if trimmed.starts_with("> ") {
        return true;
    }

    // Numbered continuation: `2. `, `10. `, etc.
    // Pattern: optional digits followed by `. ` or just digits followed by `.`
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    // Must start with a digit
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return false;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Must be followed by `. ` or be `N.` at end of line
    if i < bytes.len() && bytes[i] == b'.' {
        // `N.` (end) or `N. ` (continuation)
        if i + 1 == bytes.len() || bytes[i + 1] == b' ' {
            return true;
        }
    }

    false
}

/// Strip Stata boilerplate from log output, returning only substantive content
///
/// Removes:
/// - Lines starting with `. ` (command echo — Stata repeating what it was told)
/// - Lines that are exactly `.` (continuation of command echo)
/// - Numbered continuation lines inside loops/programs (e.g., `  2. display x`)
/// - Leading blank lines
/// - `end of do-file` marker and everything after it (including `r(CODE);`)
/// - Collapses consecutive blank lines to a single blank line
/// - Trims trailing whitespace
pub fn strip_boilerplate(log_content: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();

    for line in log_content.lines() {
        // Stop at end-of-do-file marker
        if line.trim() == "end of do-file" {
            break;
        }

        // Skip command echo lines
        let trimmed = line.trim();
        if is_command_echo(trimmed) {
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

    #[test]
    fn test_is_command_echo_standard() {
        assert!(is_command_echo(". display 1"));
        assert!(is_command_echo("."));
        assert!(is_command_echo(". foreach x of numlist 1/3 {"));
    }

    #[test]
    fn test_is_command_echo_numbered() {
        assert!(is_command_echo("2. display `x'"));
        assert!(is_command_echo("10. end"));
        assert!(is_command_echo("3."));
    }

    #[test]
    fn test_is_command_echo_continuation() {
        assert!(is_command_echo("> /\") + 1, .), \"\", .)"));
        assert!(is_command_echo("> )' _n\""));
        assert!(is_command_echo("> local x = 1"));
    }

    #[test]
    fn test_is_command_echo_not_echo() {
        assert!(!is_command_echo("hello world"));
        assert!(!is_command_echo("2"));
        assert!(!is_command_echo("r(199);"));
        assert!(!is_command_echo(""));
        assert!(!is_command_echo("123"));
        // A line like "3.14" is not command echo (no space after dot)
        assert!(!is_command_echo("3.14"));
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
