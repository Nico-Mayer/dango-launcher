//! Crashes that are not panics, written to the log.
//!
//! A panic reaches the panic hook. An access violation, a stack overflow or a
//! corrupted heap does not, and Windows does not reliably put them in the
//! event log either, so without this the process just disappears.

use std::path::Path;
use std::sync::mpsc::sync_channel;
use std::sync::Mutex;
use std::time::Duration;

use crash_handler::{make_crash_event, CrashContext, CrashEventResult, CrashHandler};
use windows_sys::Win32::Foundation::{HMODULE, MAX_PATH};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};

/// A writer stuck behind a lock the crashing thread holds must not turn the
/// crash into a hang.
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
struct Fault {
    code: u32,
    address: usize,
    thread: u32,
}

pub fn record_crashes() {
    let (faults, incoming) = sync_channel::<Fault>(1);
    let (written, done) = sync_channel::<()>(1);
    // The crashing thread may hold the heap lock or have no stack left, so it
    // only hands over numbers. This thread, started while all is well, does
    // the allocating and the writing.
    let writer = std::thread::Builder::new()
        .name("crash-log".into())
        .spawn(move || {
            while let Ok(fault) = incoming.recv() {
                crate::log::append(&describe(fault));
                let _ = written.send(());
            }
        });
    if writer.is_err() {
        return;
    }

    let done = Mutex::new(done);
    let attached = CrashHandler::attach(unsafe {
        make_crash_event(move |context: &CrashContext| {
            if faults.try_send(fault(context)).is_ok() {
                if let Ok(done) = done.try_lock() {
                    let _ = done.recv_timeout(WRITE_TIMEOUT);
                }
            }
            CrashEventResult::Handled(false)
        })
    });
    match attached {
        // Dropping the handler detaches it.
        Ok(handler) => std::mem::forget(handler),
        Err(error) => crate::log::append(&format!("crashes will not be logged: {error}")),
    }
}

fn fault(context: &CrashContext) -> Fault {
    let record = unsafe {
        context
            .exception_pointers
            .as_ref()
            .and_then(|pointers| pointers.ExceptionRecord.as_ref())
    };
    Fault {
        code: context.exception_code as u32,
        address: record.map_or(0, |record| record.ExceptionAddress as usize),
        thread: context.thread_id,
    }
}

fn describe(fault: Fault) -> String {
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    format!(
        "{now} crash 0x{:08X} at {} on thread {}",
        fault.code,
        location(fault.address),
        fault.thread
    )
}

/// The address as module and offset, which stays meaningful across runs while
/// the raw address does not.
fn location(address: usize) -> String {
    let mut module: HMODULE = std::ptr::null_mut();
    let found = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            address as *const u16,
            &mut module,
        )
    };
    if found == 0 {
        return format!("0x{address:X}");
    }
    let mut path = [0u16; MAX_PATH as usize];
    let length = unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), MAX_PATH) } as usize;
    let path = String::from_utf16_lossy(&path[..length]);
    let name = Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or(path);
    format!("{name}+0x{:X}", address - module as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_in_this_program_is_named_by_module_and_offset() {
        let here = location as *const () as usize;
        let text = location(here);
        let (name, offset) = text.split_once("+0x").expect("a module and an offset");
        assert!(name.ends_with(".exe"), "{text}");
        assert!(usize::from_str_radix(offset, 16).unwrap() < here, "{text}");
    }

    #[test]
    fn an_address_in_no_module_stays_a_number() {
        assert_eq!(location(8), "0x8");
    }
}
