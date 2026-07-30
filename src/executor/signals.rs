//! Forwarding terminal signals to Stata process groups
//!
//! Each Stata child runs in its own process group (see `runner::run_stata`), so
//! a timeout can signal the whole tree instead of only the process stacy holds
//! a handle to. That isolation costs the behavior the terminal used to provide
//! for free: Ctrl-C reaches the foreground process group, which no longer
//! contains Stata. Without forwarding, Ctrl-C would kill stacy and leave Stata
//! running — the very thing #118 is about.
//!
//! So SIGINT, SIGTERM and SIGHUP are forwarded to every live child group, and
//! then stacy dies of the same signal, keeping the exit status callers already
//! expect (130 for Ctrl-C).
//!
//! Windows needs none of this: console events go to every process attached to
//! the console, so Stata already hears Ctrl-C.

#[cfg(unix)]
use std::sync::atomic::{AtomicI32, Ordering};
#[cfg(unix)]
use std::sync::Once;

/// Live child process groups. Fixed size and lock-free: the handler runs in
/// signal context, where allocating and locking are not allowed. Parallel runs
/// are bounded by core count, so the table cannot realistically fill; a child
/// that finds no slot simply is not forwarded to.
#[cfg(unix)]
const MAX_GROUPS: usize = 128;
#[cfg(unix)]
static GROUPS: [AtomicI32; MAX_GROUPS] = [const { AtomicI32::new(0) }; MAX_GROUPS];
#[cfg(unix)]
static INSTALLED: Once = Once::new();

/// Keeps a process group in the forwarding table. Dropping it removes the
/// entry, so a reaped child's pid is never signalled again.
pub struct Registration {
    #[cfg(unix)]
    slot: Option<usize>,
}

impl Drop for Registration {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(slot) = self.slot {
            GROUPS[slot].store(0, Ordering::SeqCst);
        }
    }
}

/// Forward signals to `pgid` until the returned guard is dropped.
pub fn register(pgid: i32) -> Registration {
    #[cfg(unix)]
    {
        install_handlers();

        for (slot, entry) in GROUPS.iter().enumerate() {
            if entry
                .compare_exchange(0, pgid, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return Registration { slot: Some(slot) };
            }
        }

        Registration { slot: None }
    }

    #[cfg(not(unix))]
    {
        let _ = pgid;
        Registration {}
    }
}

/// Install the forwarding handlers once per process.
#[cfg(unix)]
fn install_handlers() {
    INSTALLED.call_once(|| {
        for signum in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            // Through a fn pointer: casting the function item straight to an
            // integer is what `libc::signal` wants, but not what it means.
            let handler = forward as extern "C" fn(libc::c_int);
            // SAFETY: `forward` only performs async-signal-safe work.
            unsafe {
                libc::signal(signum, handler as libc::sighandler_t);
            }
        }
    });
}

/// Signal handler: pass the signal on to every live child group, then die of
/// it. Only atomic loads, `kill(2)` and `raise(3)` — all async-signal-safe.
#[cfg(unix)]
extern "C" fn forward(signum: libc::c_int) {
    for entry in GROUPS.iter() {
        let pgid = entry.load(Ordering::SeqCst);
        if pgid > 0 {
            // SAFETY: a negative pid signals the whole process group.
            unsafe {
                libc::kill(-pgid, signum);
            }
        }
    }

    // Restore the default disposition and re-raise, so stacy's own exit status
    // stays what it was before this module existed.
    unsafe {
        libc::signal(signum, libc::SIG_DFL);
        libc::raise(signum);
    }
}
