//! A private session bus: a `dbus-daemon` with its own config in a scratch folder.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

const CONFIG: &str = r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:path=SOCKET</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*"/>
    <allow receive_sender="*"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#;

/// A running private bus; the daemon is killed when this drops.
pub struct PrivateBus {
    daemon: Child,
    address: String,
    _scratch: tempfile::TempDir,
}

impl PrivateBus {
    /// Starts a bus, or says why not and returns `None` so the test ends early.
    pub fn start(test: &str) -> Option<PrivateBus> {
        let scratch = tempfile::tempdir().unwrap();
        let socket = scratch.path().join("bus.sock");
        let config = scratch.path().join("bus.conf");
        std::fs::write(&config, CONFIG.replace("SOCKET", &socket.to_string_lossy())).unwrap();
        let spawned = Command::new("dbus-daemon")
            .arg("--nofork")
            .arg("--print-address=1")
            .arg(format!("--config-file={}", config.display()))
            .env_clear()
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let mut daemon = match spawned {
            Ok(daemon) => daemon,
            Err(error) => {
                eprintln!("skipping {test}: dbus-daemon cannot be started ({error})");
                return None;
            }
        };
        let mut address = String::new();
        if let Some(stdout) = daemon.stdout.take() {
            let _ = BufReader::new(stdout).read_line(&mut address);
        }
        // Built before the check, so a failed start still kills the daemon on drop.
        let bus = PrivateBus {
            daemon,
            address: address.trim().to_owned(),
            _scratch: scratch,
        };
        if !bus.address.starts_with("unix:") {
            eprintln!("skipping {test}: dbus-daemon printed no address");
            return None;
        }
        Some(bus)
    }

    /// A new connection to the bus.
    pub async fn connect(&self) -> zbus::Connection {
        zbus::connection::Builder::address(self.address.as_str())
            .unwrap()
            .build()
            .await
            .unwrap()
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}
