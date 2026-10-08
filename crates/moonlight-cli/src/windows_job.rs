//! Keeps a Windows target and its descendants in one kill-on-close job.
use std::{
    io,
    mem::{size_of, zeroed},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr,
};
use tokio::process::Child;
use windows_sys::Win32::{
    Foundation::INVALID_HANDLE_VALUE,
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        },
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
    },
};

pub(crate) struct WindowsJob {
    _handle: OwnedHandle,
}

impl WindowsJob {
    /// The child must have been created suspended so it cannot escape before assignment.
    pub(crate) fn attach_and_resume(child: &Child) -> io::Result<Self> {
        // SAFETY: null attributes/name create an unnamed, non-inheritable job. The returned
        // handle is checked before being transferred to its sole OwnedHandle owner.
        let raw = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if raw.is_null() {
            return Err(io::Error::last_os_error());
        }
        let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
        // SAFETY: zero is valid for this Win32 structure; its size and pointer remain valid
        // for the synchronous SetInformationJobObject call.
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                handle.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let process = child
            .raw_handle()
            .ok_or_else(|| io::Error::other("suspended target has no process handle"))?;
        // SAFETY: both handles are live; the child has not resumed and cannot have spawned descendants.
        if unsafe { AssignProcessToJobObject(handle.as_raw_handle(), process) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let job = Self { _handle: handle };
        resume_initial_thread(
            child
                .id()
                .ok_or_else(|| io::Error::other("suspended target has no PID"))?,
        )?;
        Ok(job)
    }
}

fn resume_initial_thread(pid: u32) -> io::Result<()> {
    // SAFETY: the snapshot owns a new kernel handle, checked before transfer.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
    // SAFETY: zero initializes every documented field; dwSize enables the Win32 output contract.
    let mut entry: THREADENTRY32 = unsafe { zeroed() };
    entry.dwSize = size_of::<THREADENTRY32>() as u32;
    let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) };
    while found != 0 {
        if entry.th32OwnerProcessID == pid {
            // SAFETY: OpenThread creates an owned handle to this suspended target's initial thread.
            let raw = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if raw.is_null() {
                return Err(io::Error::last_os_error());
            }
            let thread = unsafe { OwnedHandle::from_raw_handle(raw) };
            if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
                return Err(io::Error::last_os_error());
            }
            return Ok(());
        }
        found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) };
    }
    Err(io::Error::other(
        "suspended target's initial thread was not found",
    ))
}
