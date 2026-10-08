//! Adopt and reap terminated target descendants even when container PID 1 does not reap.
use std::{
    collections::BTreeSet,
    io,
    sync::Mutex,
    time::{Duration, Instant},
};

static TERMINATED_GROUPS: Mutex<BTreeSet<i32>> = Mutex::new(BTreeSet::new());

pub(crate) fn adopt_descendants() -> io::Result<()> {
    // SAFETY: PR_SET_CHILD_SUBREAPER accepts an integer flag and no pointer arguments.
    let result = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(crate) fn record_terminated_group(id: u32) {
    if let Ok(id) = i32::try_from(id) {
        TERMINATED_GROUPS
            .lock()
            .expect("target group mutex poisoned")
            .insert(id);
    }
}

pub(crate) async fn reap_terminated_groups() -> anyhow::Result<()> {
    let groups = std::mem::take(
        &mut *TERMINATED_GROUPS
            .lock()
            .expect("target group mutex poisoned"),
    );
    tokio::task::spawn_blocking(move || reap_groups(groups)).await??;
    Ok(())
}

fn reap_groups(mut groups: BTreeSet<i32>) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_millis(100);
    while !groups.is_empty() {
        for id in groups.clone() {
            loop {
                let mut status = 0;
                // SAFETY: status is a valid writable integer. The negative PID selects only
                // children in this CLI's recorded, terminated target process group.
                let result = unsafe { libc::waitpid(-id, &mut status, libc::WNOHANG) };
                if result > 0 {
                    continue;
                }
                if result == 0 {
                    break;
                }
                let error = io::Error::last_os_error();
                match error.raw_os_error() {
                    Some(libc::ECHILD) => {
                        groups.remove(&id);
                        break;
                    }
                    Some(libc::EINTR) => continue,
                    _ => return Err(error),
                }
            }
        }
        if groups.is_empty() {
            break;
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "target descendants were not reaped before the cleanup deadline",
            ));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}
