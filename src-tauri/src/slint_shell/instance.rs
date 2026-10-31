//! One Dango per user. A second launch asks the running one to show the
//! launcher and exits, so starting Dango again is a way to summon it.

use std::io::{self, BufRead, BufReader, Write};

use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions, Stream};
use single_instance::SingleInstance;

pub const NAME: &str = "com.nimayer.dango";

const SHOW: &str = "show";

/// Held for as long as this process is the one Dango. `None` when another
/// instance already runs.
pub fn claim() -> Option<SingleInstance> {
    let lock = SingleInstance::new(NAME).ok()?;
    lock.is_single().then_some(lock)
}

/// Answers a second launch on its own thread.
pub fn listen(name: &str, show: impl Fn() + Send + 'static) -> io::Result<()> {
    let name = socket_name(name)?;
    let listener = ListenerOptions::new()
        .name(name)
        .try_overwrite(true)
        .create_sync()?;
    std::thread::Builder::new()
        .name("dango-instance".into())
        .spawn(move || {
            for connection in listener.incoming().filter_map(Result::ok) {
                let mut line = String::new();
                if BufReader::new(connection).read_line(&mut line).is_ok() && line.trim() == SHOW {
                    show();
                }
            }
        })?;
    Ok(())
}

/// Asks the running instance to show the launcher.
pub fn signal(name: &str) -> io::Result<()> {
    let mut stream = Stream::connect(socket_name(name)?)?;
    stream.write_all(format!("{SHOW}\n").as_bytes())
}

fn socket_name(name: &str) -> io::Result<interprocess::local_socket::Name<'static>> {
    format!("{name}.sock").to_ns_name::<GenericNamespaced>()
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn a_second_instance_reaches_the_first() {
        let name = format!("dango-test-{}", std::process::id());
        let (shown, received) = mpsc::channel();
        listen(&name, move || {
            let _ = shown.send(());
        })
        .unwrap();
        signal(&name).unwrap();
        assert!(received.recv_timeout(Duration::from_secs(5)).is_ok());
    }

    #[test]
    fn nothing_answers_when_no_instance_runs() {
        let name = format!("dango-test-absent-{}", std::process::id());
        assert!(signal(&name).is_err());
    }
}
