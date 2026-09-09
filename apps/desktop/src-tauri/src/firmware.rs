//! Native-only firmware flashing workflow.
//!
//! The webview can only request a flash of the fixed firmware bundled into this binary. Port names,
//! filesystem paths, and `espflash` output remain inside the native process.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Maximum time allowed for one `espflash` process before it is terminated.
pub const FLASH_TIMEOUT: Duration = Duration::from_secs(90);
/// Maximum time to wait for the flashed device to reconnect and complete a handshake.
pub const RECONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// The fixed firmware artifact embedded by `build.rs`; an empty file means this build cannot flash.
pub static BUNDLED_FIRMWARE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/kivori-firmware.elf"));

/// User-visible firmware flashing phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FirmwarePhase {
    /// No flash has been requested.
    Idle,
    /// Native validation has accepted a queued flash request.
    Preparing,
    /// `espflash` owns the port and is programming the fixed artifact.
    Flashing,
    /// Programming finished; the desktop is waiting for the same device to handshake again.
    Reconnecting,
    /// The same device completed its post-flash handshake.
    Succeeded,
    /// Flashing or required reconnect verification failed.
    Failed,
}

/// Safe status projection returned to the webview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareStatus {
    /// Whether this app build contains a valid fixed firmware image.
    pub available: bool,
    /// Current native workflow phase.
    pub phase: FirmwarePhase,
    /// Human-readable status with no port, path, or process output.
    pub message: String,
    /// Bundled image byte length, or zero when unavailable.
    pub image_size: u64,
}

/// Result of a completed programming attempt, consumed by the device runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeTarget {
    /// Resume ordinary candidate discovery after a failed/cancelled flash.
    Discovery,
    /// Reopen only this exact previously-connected port while awaiting handshake verification.
    SamePort(String),
}

/// Small, testable flashing state machine. It retains native-only target data but exposes only a
/// [`FirmwareStatus`] projection to the IPC boundary.
#[derive(Debug, Clone)]
pub struct FlashWorkflow {
    status: FirmwareStatus,
    target_port: Option<String>,
    expected_device_hash: Option<String>,
}

impl FlashWorkflow {
    /// Creates a workflow around the bundled image availability and its byte length.
    #[must_use]
    pub fn new(available: bool, image_size: u64) -> Self {
        let message = if available {
            "Firmware ready to flash.".to_string()
        } else {
            "Firmware is unavailable in this application build.".to_string()
        };
        Self {
            status: FirmwareStatus {
                available,
                phase: FirmwarePhase::Idle,
                message,
                image_size: if available { image_size } else { 0 },
            },
            target_port: None,
            expected_device_hash: None,
        }
    }

    /// Returns the safe status projection.
    #[must_use]
    pub fn status(&self) -> &FirmwareStatus {
        &self.status
    }

    /// Whether this workflow owns the serial connection or is waiting to verify its return.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        matches!(
            self.status.phase,
            FirmwarePhase::Preparing | FirmwarePhase::Flashing | FirmwarePhase::Reconnecting
        )
    }

    /// The native-only port reserved for the reconnect check. It is never part of [`FirmwareStatus`].
    #[must_use]
    pub(crate) fn target_port(&self) -> Option<&str> {
        self.target_port.as_deref()
    }

    /// Validates and reserves the currently connected native target. Returns the port for the device
    /// thread only; callers must never expose it to IPC.
    pub fn request(
        &mut self,
        connected: bool,
        port: Option<&str>,
        device_hash: Option<&str>,
    ) -> Result<String, String> {
        if !self.status.available {
            return Err("Firmware is unavailable in this application build.".to_string());
        }
        if self.is_busy() {
            return Err("A firmware update is already in progress.".to_string());
        }
        if !connected {
            return Err("Connect a compatible device before flashing firmware.".to_string());
        }
        let port =
            port.ok_or_else(|| "The connected device is no longer available.".to_string())?;
        let device_hash = device_hash.ok_or_else(|| {
            "The connected device could not be verified for flashing.".to_string()
        })?;
        self.target_port = Some(port.to_string());
        self.expected_device_hash = Some(device_hash.to_string());
        self.status.phase = FirmwarePhase::Preparing;
        self.status.message = "Preparing firmware update.".to_string();
        Ok(port.to_string())
    }

    /// Marks that the serial session has been released and the flasher now owns the port.
    pub fn mark_flashing(&mut self) {
        self.status.phase = FirmwarePhase::Flashing;
        self.status.message = "Flashing firmware.".to_string();
    }

    /// Marks that a queued update lost its device before the flash process could acquire the port.
    pub fn fail_preparation(&mut self) {
        self.status.phase = FirmwarePhase::Failed;
        self.status.message =
            "The connected device is no longer available for firmware flashing.".to_string();
        self.target_port = None;
        self.expected_device_hash = None;
    }

    /// Completes process execution and selects the safe connection-resume mode.
    pub fn finish(&mut self, result: Result<(), impl AsRef<str>>) -> ResumeTarget {
        match result {
            Ok(()) => {
                self.status.phase = FirmwarePhase::Reconnecting;
                self.status.message =
                    "Firmware flashed. Reconnecting to device for verification.".to_string();
                ResumeTarget::SamePort(self.target_port.clone().unwrap_or_default())
            }
            Err(reason) => {
                self.status.phase = FirmwarePhase::Failed;
                self.status.message = safe_failure_message(reason.as_ref()).to_string();
                self.target_port = None;
                self.expected_device_hash = None;
                ResumeTarget::Discovery
            }
        }
    }

    /// Accepts the post-flash handshake only if both its port and already-verified device identity
    /// match the original native target.
    pub fn handshake(&mut self, port: &str, device_hash: &str, compatible: bool) -> bool {
        if self.status.phase != FirmwarePhase::Reconnecting
            || !compatible
            || self.target_port.as_deref() != Some(port)
            || self.expected_device_hash.as_deref() != Some(device_hash)
        {
            return false;
        }
        self.status.phase = FirmwarePhase::Succeeded;
        self.status.message = "Firmware update verified.".to_string();
        self.target_port = None;
        self.expected_device_hash = None;
        true
    }

    /// Fails a pending post-flash reconnect without treating any later discovered device as success.
    pub fn reconnect_timed_out(&mut self) {
        if self.status.phase == FirmwarePhase::Reconnecting {
            self.status.phase = FirmwarePhase::Failed;
            self.status.message =
                "Firmware flashed, but the device could not be verified after reconnecting."
                    .to_string();
            self.target_port = None;
            self.expected_device_hash = None;
        }
    }
}

fn safe_failure_message(reason: &str) -> &'static str {
    match reason {
        "Firmware is unavailable in this application build." => {
            "Firmware is unavailable in this application build."
        }
        "Firmware flashing was cancelled." => "Firmware flashing was cancelled.",
        "Firmware flashing timed out." => "Firmware flashing timed out.",
        "espflash could not flash the firmware." => "espflash could not flash the firmware.",
        "Unable to monitor espflash." => "Unable to monitor espflash.",
        "Unable to prepare the bundled firmware image." => {
            "Unable to prepare the bundled firmware image."
        }
        "Unable to start espflash. Check that espflash 4.5 is installed." => {
            "Unable to start espflash. Check that espflash 4.5 is installed."
        }
        _ if reason.starts_with("espflash 4.5 is not installed.") => {
            "espflash 4.5 is not installed. Install it with cargo install espflash --version 4.5.0."
        }
        _ => "Firmware flashing failed.",
    }
}

/// Returns whether the bundled artifact is an ELF32 little-endian RISC-V executable.
#[must_use]
pub fn bundled_image_available() -> bool {
    is_elf32_riscv(BUNDLED_FIRMWARE)
}

/// Returns the initial safe status for the currently embedded artifact.
#[must_use]
pub fn initial_status() -> FirmwareStatus {
    FlashWorkflow::new(bundled_image_available(), BUNDLED_FIRMWARE.len() as u64)
        .status
        .clone()
}

/// Runs the fixed bundled image through `espflash` for the given native-owned port.
///
/// Process output is discarded so raw port and tool output cannot reach the webview. The child is
/// killed and reaped on timeout or application shutdown.
pub fn flash_bundled(port: &str, cancel: &AtomicBool) -> Result<(), String> {
    if !bundled_image_available() {
        return Err("Firmware is unavailable in this application build.".to_string());
    }
    let tool = find_espflash().ok_or_else(|| {
        "espflash 4.5 is not installed. Install it with `cargo install espflash --version 4.5.0`."
            .to_string()
    })?;
    let temp = write_temp_image(BUNDLED_FIRMWARE)?;
    let result = run_espflash(&tool, port, &temp, cancel);
    let _ = std::fs::remove_file(&temp);
    result
}

fn is_elf32_riscv(bytes: &[u8]) -> bool {
    bytes.len() >= 20
        && bytes.starts_with(b"\x7fELF")
        && bytes[4] == 1
        && bytes[5] == 1
        && bytes[18] == 0xF3
        && bytes[19] == 0
}

fn find_espflash() -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "espflash.exe"
    } else {
        "espflash"
    };
    let mut candidates = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|dir| dir.join(exe))
        .collect::<Vec<_>>();
    if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        candidates.push(PathBuf::from(home).join(".cargo").join("bin").join(exe));
    }
    candidates.into_iter().find(|path| path.is_file())
}

fn write_temp_image(image: &[u8]) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "kivori-firmware-{}-{stamp}.elf",
        std::process::id()
    ));
    std::fs::write(&path, image)
        .map_err(|_| "Unable to prepare the bundled firmware image.".to_string())?;
    Ok(path)
}

fn run_espflash(tool: &Path, port: &str, image: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let mut command = Command::new(tool);
    command
        .arg("flash")
        .arg("--port")
        .arg(port)
        .arg("--chip")
        .arg("esp32c3")
        .arg("--non-interactive")
        .arg("--skip-update-check")
        .arg(image)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().map_err(|_| {
        "Unable to start espflash. Check that espflash 4.5 is installed.".to_string()
    })?;
    let started = Instant::now();
    let result = loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = child.kill();
            break Err("Firmware flashing was cancelled.".to_string());
        }
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break Ok(()),
            Ok(Some(_)) => break Err("espflash could not flash the firmware.".to_string()),
            Ok(None) if started.elapsed() >= FLASH_TIMEOUT => {
                let _ = child.kill();
                break Err("Firmware flashing timed out.".to_string());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => {
                // A failed status probe does not reap the child. Terminate it before the unconditional
                // `wait` below so a broken process handle cannot strand the device runtime.
                let _ = child.kill();
                break Err("Unable to monitor espflash.".to_string());
            }
        }
    };
    let _ = child.wait();
    result
}
