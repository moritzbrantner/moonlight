//! A command-local subreaper owns descendants across process-group/session changes.
use crate::types::{CommandForm, TargetCommand};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io::{self, BufRead, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            net::UnixStream as StdUnixStream,
            process::{CommandExt, ExitStatusExt},
        },
    },
    process::{ExitStatus, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    process::{Child, Command},
};

const SUPERVISOR_ARGUMENT: &str = "--__moonlight-supervise-target-v1";
const CONTROL_FD: i32 = 3;
static DROPPED_SUPERVISORS: Mutex<BTreeSet<i32>> = Mutex::new(BTreeSet::new());

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum Reply {
    Started,
    SpawnFailed { message: String },
    Finished { status: i32 },
}

pub(crate) struct SupervisorControl {
    channel: Option<BufReader<UnixStream>>,
    pid: Option<i32>,
}

impl SupervisorControl {
    async fn reply(&mut self) -> io::Result<Reply> {
        let channel = self.channel.as_mut().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "target supervisor channel closed",
            )
        })?;
        let mut line = String::new();
        if channel.read_line(&mut line).await? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "target supervisor exited without a result",
            ));
        }
        serde_json::from_str(&line).map_err(io::Error::other)
    }

    pub(crate) async fn wait(&mut self) -> io::Result<ExitStatus> {
        match self.reply().await? {
            Reply::Finished { status } => Ok(ExitStatus::from_raw(status)),
            _ => Err(io::Error::other("unexpected target supervisor reply")),
        }
    }

    pub(crate) fn terminate(&mut self) {
        // EOF is the cancellation request. The payload never inherits this descriptor.
        self.channel.take();
    }

    pub(crate) fn disarm(&mut self) {
        self.pid = None;
    }
}

impl Drop for SupervisorControl {
    fn drop(&mut self) {
        self.terminate();
        if let Some(pid) = self.pid.take() {
            DROPPED_SUPERVISORS
                .lock()
                .expect("supervisor mutex poisoned")
                .insert(pid);
        }
    }
}

pub(crate) async fn spawn_target(target: &TargetCommand) -> io::Result<(Child, SupervisorControl)> {
    let (parent, worker) = StdUnixStream::pair()?;
    let worker_fd = worker.as_raw_fd();
    parent.set_nonblocking(true)?;
    let parent = UnixStream::from_std(parent)?;
    let mut request = serde_json::to_vec(target).map_err(io::Error::other)?;
    request.push(b'\n');
    let mut command = Command::new(std::env::current_exe()?);
    command.as_std_mut().process_group(0);
    command
        .arg(SUPERVISOR_ARGUMENT)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false);
    // SAFETY: the child-side closure calls only descriptor syscalls before exec. The
    // worker stream remains alive until spawn returns; descriptor 3 is private to it.
    unsafe {
        command.as_std_mut().pre_exec(move || {
            if libc::dup2(worker_fd, CONTROL_FD) == -1
                || libc::fcntl(CONTROL_FD, libc::F_SETFD, 0) == -1
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child = command.spawn()?;
    drop(worker);
    let mut control = SupervisorControl {
        channel: Some(BufReader::new(parent)),
        pid: child.id().and_then(|pid| i32::try_from(pid).ok()),
    };
    control
        .channel
        .as_mut()
        .expect("new supervisor channel")
        .get_mut()
        .write_all(&request)
        .await?;
    match control.reply().await? {
        Reply::Started => Ok((child, control)),
        Reply::SpawnFailed { message } => Err(io::Error::other(message)),
        _ => Err(io::Error::other("unexpected target startup reply")),
    }
}

pub(crate) fn is_supervisor_request() -> bool {
    let mut args = std::env::args_os();
    args.next();
    args.next().is_some_and(|arg| arg == SUPERVISOR_ARGUMENT) && args.next().is_none()
}

pub(crate) fn run_supervisor() -> io::Result<()> {
    // Validate the private descriptor before taking ownership, including direct
    // unsupported invocations of the hidden launcher argument.
    // SAFETY: F_GETFD takes only an integer descriptor and no pointer arguments.
    if unsafe { libc::fcntl(CONTROL_FD, libc::F_GETFD) } == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: descriptor 3 is open and owned exclusively by this single-threaded
    // worker process. peer_addr verifies its Unix socket kind before protocol IO.
    let channel = unsafe { StdUnixStream::from_raw_fd(CONTROL_FD) };
    channel.peer_addr()?;
    // SAFETY: these calls take integer arguments only. Close-on-exec prevents the
    // target from inheriting its supervisor's control channel.
    if unsafe { libc::fcntl(CONTROL_FD, libc::F_SETFD, libc::FD_CLOEXEC) } == -1
        || unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } == -1
    {
        return Err(io::Error::last_os_error());
    }
    let mut reader = std::io::BufReader::new(channel);
    let mut request = String::new();
    if reader.read_line(&mut request)? == 0 {
        return Ok(());
    }
    let target: TargetCommand = serde_json::from_str(&request).map_err(io::Error::other)?;
    let mut channel = reader.into_inner();
    let mut command = match target.form {
        CommandForm::Shell(script) => {
            let mut command = std::process::Command::new("sh");
            command.args(["-lc", &script]);
            command
        }
        CommandForm::Argv(argv) => {
            let mut command = std::process::Command::new(&argv[0]);
            command.args(&argv[1..]);
            command
        }
    };
    if let Some(cwd) = target.cwd {
        command.current_dir(cwd);
    }
    command
        .envs(target.env)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            write_reply(
                &mut channel,
                &Reply::SpawnFailed {
                    message: error.to_string(),
                },
            )?;
            return Ok(());
        }
    };
    let root_pid = child.id() as i32;
    let mut owned_children = OwnedChildren(true);
    // std::process::Child has no kill-on-drop; this worker exclusively waits for all
    // of its children with waitpid and communicates the root's original raw status.
    drop(child);
    write_reply(&mut channel, &Reply::Started)?;
    channel.set_nonblocking(true)?;
    let mut root_finished = false;
    let mut cleanup_deadline = None;
    loop {
        let mut byte = [0];
        match channel.read(&mut byte) {
            Ok(0) => {
                cleanup_deadline.get_or_insert_with(|| Instant::now() + Duration::from_millis(400));
            }
            Ok(_) => return Err(io::Error::other("unexpected supervisor control data")),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
        if cleanup_deadline.is_some() {
            kill_owned_children()?;
        }
        let mut children_remain = false;
        loop {
            let mut status = 0;
            // SAFETY: this dedicated single-threaded worker owns every child; status
            // points to a writable integer. No concurrent Child waiter exists here.
            let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
            if pid > 0 {
                if pid == root_pid && !root_finished {
                    root_finished = true;
                    if cleanup_deadline.is_none() {
                        write_reply(&mut channel, &Reply::Finished { status })?;
                        // Preserve the original pipe-drain semantics: only descendants
                        // retaining output handles can keep the parent's stream open.
                        // SAFETY: these are this worker's own inherited output descriptors.
                        unsafe {
                            libc::close(libc::STDOUT_FILENO);
                            libc::close(libc::STDERR_FILENO);
                        }
                    }
                }
                continue;
            }
            if pid == 0 {
                children_remain = true;
                break;
            }
            let error = io::Error::last_os_error();
            match error.raw_os_error() {
                Some(libc::ECHILD) => break,
                Some(libc::EINTR) => continue,
                _ => return Err(error),
            }
        }
        if cleanup_deadline.is_some() && !children_remain {
            owned_children.0 = false;
            return Ok(());
        }
        if cleanup_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "target descendant cleanup exceeded its deadline",
            ));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

struct OwnedChildren(bool);

impl Drop for OwnedChildren {
    fn drop(&mut self) {
        if !self.0 {
            return;
        }
        // Protocol failures during startup/exit must clean up just like EOF cancellation.
        let deadline = Instant::now() + Duration::from_millis(400);
        while Instant::now() < deadline {
            if kill_owned_children().is_err() {
                return;
            }
            loop {
                let mut status = 0;
                // SAFETY: this single-threaded supervisor exclusively owns all children.
                let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
                if pid > 0 {
                    continue;
                }
                if pid == -1 && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD) {
                    return;
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

fn kill_owned_children() -> io::Result<()> {
    for pid in owned_child_pids()? {
        // SAFETY: these PIDs are unreaped direct children of this worker. Killing
        // their parents adopts further descendants here, including setsid children.
        if unsafe { libc::kill(pid, libc::SIGKILL) } == -1 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }
    }
    Ok(())
}

fn owned_child_pids() -> io::Result<Vec<i32>> {
    let worker = std::process::id();
    match std::fs::read_to_string(format!("/proc/self/task/{worker}/children")) {
        Ok(children) => children
            .split_whitespace()
            .map(|pid| pid.parse().map_err(io::Error::other))
            .collect(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // The children entry requires optional kernel configuration. Standard
            // procfs stat entries still identify this supervisor's direct children.
            let mut children = Vec::new();
            for entry in std::fs::read_dir("/proc")? {
                let entry = entry?;
                let Some(pid) = entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse::<i32>().ok())
                else {
                    continue;
                };
                let Ok(stat) = std::fs::read(entry.path().join("stat")) else {
                    continue;
                };
                // Process names may contain spaces, parentheses or non-UTF8 bytes;
                // only the numeric fields after the final name delimiter are parsed.
                let Some(end) = stat.iter().rposition(|byte| *byte == b')') else {
                    continue;
                };
                let parent = stat[end + 1..]
                    .split(|byte| byte.is_ascii_whitespace())
                    .filter(|field| !field.is_empty())
                    .nth(1)
                    .and_then(|field| std::str::from_utf8(field).ok())
                    .and_then(|parent| parent.parse::<u32>().ok());
                if parent == Some(worker) {
                    children.push(pid);
                }
            }
            Ok(children)
        }
        Err(error) => Err(error),
    }
}

fn write_reply(channel: &mut StdUnixStream, reply: &Reply) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(reply).map_err(io::Error::other)?;
    bytes.push(b'\n');
    channel.write_all(&bytes)
}

pub(crate) async fn reap_dropped_supervisors() -> anyhow::Result<()> {
    let mut pids = std::mem::take(
        &mut *DROPPED_SUPERVISORS
            .lock()
            .expect("supervisor mutex poisoned"),
    );
    tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_secs(1);
        while !pids.is_empty() {
            for pid in pids.clone() {
                let mut status = 0;
                // SAFETY: only recorded, dropped supervisor Child handles are reaped.
                let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
                if result == pid
                    || (result == -1
                        && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD))
                {
                    pids.remove(&pid);
                }
            }
            if pids.is_empty() {
                break;
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "target supervisor did not finish cleanup",
                ));
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    })
    .await??;
    Ok(())
}
