//! Inter-process communication for wallpaper control
//!
//! Uses platform-specific local IPC between the CLI and running wallpaper instances.
//! Protocol: Simple text-based commands (STOP:N, STOP:ALL, PING)

#[cfg(unix)]
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(unix)]
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// IPC command sent from client to server
#[derive(Debug, Clone, PartialEq)]
pub enum IpcCommand {
    /// Stop wallpaper on specific display
    Stop(u32),
    /// Stop all wallpapers
    StopAll,
    /// Health check
    Ping,
}

impl IpcCommand {
    /// Parse a command from string
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s == "PING" {
            Some(IpcCommand::Ping)
        } else if s == "STOP:ALL" {
            Some(IpcCommand::StopAll)
        } else if let Some(num_str) = s.strip_prefix("STOP:") {
            num_str.parse::<u32>().ok().map(IpcCommand::Stop)
        } else {
            None
        }
    }

    /// Convert command to string for transmission
    pub fn to_string(&self) -> String {
        match self {
            IpcCommand::Stop(n) => format!("STOP:{}", n),
            IpcCommand::StopAll => "STOP:ALL".to_string(),
            IpcCommand::Ping => "PING".to_string(),
        }
    }
}

/// IPC response sent from server to client
#[derive(Debug, Clone, PartialEq)]
pub enum IpcResponse {
    /// Command succeeded
    Ok,
    /// Command succeeded with count (for STOP:ALL)
    OkCount(u32),
    /// Health check response
    Pong,
    /// Error occurred
    Error(String),
}

impl IpcResponse {
    /// Parse a response from string
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s == "OK" {
            Some(IpcResponse::Ok)
        } else if s == "PONG" {
            Some(IpcResponse::Pong)
        } else if let Some(num_str) = s.strip_prefix("OK:") {
            num_str.parse::<u32>().ok().map(IpcResponse::OkCount)
        } else if let Some(msg) = s.strip_prefix("ERR:") {
            Some(IpcResponse::Error(msg.to_string()))
        } else {
            None
        }
    }

    /// Convert response to string for transmission
    pub fn to_string(&self) -> String {
        match self {
            IpcResponse::Ok => "OK".to_string(),
            IpcResponse::OkCount(n) => format!("OK:{}", n),
            IpcResponse::Pong => "PONG".to_string(),
            IpcResponse::Error(msg) => format!("ERR:{}", msg),
        }
    }
}

/// Named pipe path for IPC
#[cfg(target_os = "windows")]
pub const PIPE_NAME: &str = r"\\.\pipe\webwallpaper_control";

#[cfg(unix)]
fn socket_path() -> PathBuf {
    std::env::temp_dir()
        .join("webwallpaper")
        .join("webwallpaper_control.sock")
}

/// IPC server that listens for commands
pub struct IpcServer {
    /// Thread handle for the listener
    _thread_handle: Option<JoinHandle<()>>,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
    /// Receiver for commands from the IPC thread
    command_rx: Option<Receiver<IpcCommand>>,
}

impl IpcServer {
    /// Create a new IPC server
    pub fn new() -> Self {
        Self {
            _thread_handle: None,
            shutdown: Arc::new(AtomicBool::new(false)),
            command_rx: None,
        }
    }

    /// Start the IPC server in a background thread
    #[cfg(target_os = "windows")]
    pub fn start(&mut self) -> io::Result<()> {
        use interprocess::os::windows::named_pipe::{pipe_mode, PipeListenerOptions, PipeMode};

        let shutdown = self.shutdown.clone();
        let (tx, rx) = mpsc::channel::<IpcCommand>();
        self.command_rx = Some(rx);

        // Create the named pipe listener with correct type parameters
        // Rm = pipe_mode::Bytes (receive mode), Sm = pipe_mode::Bytes (send mode)
        let listener = PipeListenerOptions::new()
            .mode(PipeMode::Bytes)
            .path(PIPE_NAME)
            .create::<pipe_mode::Bytes, pipe_mode::Bytes>()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let handle = thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                // Accept connection
                match listener.accept() {
                    Ok(stream) => {
                        // Split into reader and writer
                        let (recv_half, send_half) = stream.split();
                        let mut reader = BufReader::new(recv_half);
                        let mut line = String::new();

                        if reader.read_line(&mut line).is_ok() {
                            if let Some(cmd) = IpcCommand::parse(&line) {
                                // Determine response based on command
                                let response = match &cmd {
                                    IpcCommand::Ping => IpcResponse::Pong,
                                    IpcCommand::Stop(_) => IpcResponse::Ok,
                                    IpcCommand::StopAll => IpcResponse::OkCount(1),
                                };

                                // Send command to main thread
                                let _ = tx.send(cmd);

                                // Send response
                                let response_str = format!("{}\n", response.to_string());
                                let mut writer = send_half;
                                let _ = writer.write_all(response_str.as_bytes());
                                let _ = writer.flush();
                            }
                        }
                    }
                    Err(_) => {
                        // Small delay before retry
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });

        self._thread_handle = Some(handle);
        Ok(())
    }

    /// Start the IPC server in a background thread
    #[cfg(unix)]
    pub fn start(&mut self) -> io::Result<()> {
        let shutdown = self.shutdown.clone();
        let (tx, rx) = mpsc::channel::<IpcCommand>();
        self.command_rx = Some(rx);

        let path = socket_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if path.exists() {
            let _ = fs::remove_file(&path);
        }

        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;

        let handle = thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = handle_unix_client(stream, &tx);
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }

            let _ = fs::remove_file(&path);
        });

        self._thread_handle = Some(handle);
        Ok(())
    }

    /// Get the command receiver
    pub fn command_receiver(&mut self) -> Option<Receiver<IpcCommand>> {
        self.command_rx.take()
    }

    /// Signal the server to shutdown
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Check if shutdown has been requested
    #[allow(dead_code)]
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
}

impl Default for IpcServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// IPC client for sending commands to running instances
pub struct IpcClient;

impl IpcClient {
    /// Send a command to the IPC server and get response
    #[cfg(target_os = "windows")]
    pub fn send_command(command: &IpcCommand) -> io::Result<IpcResponse> {
        use interprocess::os::windows::named_pipe::{pipe_mode, PipeStream};

        // Connect to the named pipe
        let stream =
            PipeStream::<pipe_mode::Bytes, pipe_mode::Bytes>::connect_by_path(PIPE_NAME)
                .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;

        // Split into reader and writer
        let (recv_half, send_half) = stream.split();

        // Send command
        let cmd_str = format!("{}\n", command.to_string());
        let mut writer = send_half;
        writer.write_all(cmd_str.as_bytes())?;
        writer.flush()?;

        // Read response
        let mut reader = BufReader::new(recv_half);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;

        IpcResponse::parse(&response_line)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid response"))
    }

    /// Send a command to the IPC server and get response
    #[cfg(unix)]
    pub fn send_command(command: &IpcCommand) -> io::Result<IpcResponse> {
        let path = socket_path();
        let stream = UnixStream::connect(&path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("Failed to connect to {:?}: {}", path, e),
            )
        })?;

        send_and_receive(stream, command)
    }

    /// Check if the IPC server is running
    #[allow(dead_code)]
    pub fn ping() -> bool {
        matches!(Self::send_command(&IpcCommand::Ping), Ok(IpcResponse::Pong))
    }

    /// Send stop command for a specific display
    pub fn stop_display(display_index: u32) -> io::Result<IpcResponse> {
        Self::send_command(&IpcCommand::Stop(display_index))
    }

    /// Send stop all command
    pub fn stop_all() -> io::Result<IpcResponse> {
        Self::send_command(&IpcCommand::StopAll)
    }
}

#[cfg(unix)]
fn send_and_receive<T>(stream: T, command: &IpcCommand) -> io::Result<IpcResponse>
where
    T: io::Read + Write,
{
    let mut writer = stream;
    let cmd_str = format!("{}\n", command.to_string());
    writer.write_all(cmd_str.as_bytes())?;
    writer.flush()?;

    let mut reader = BufReader::new(writer);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    IpcResponse::parse(&response_line)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid response"))
}

#[cfg(unix)]
fn handle_unix_client(stream: UnixStream, tx: &mpsc::Sender<IpcCommand>) -> io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    if let Some(cmd) = IpcCommand::parse(&line) {
        let response = match &cmd {
            IpcCommand::Ping => IpcResponse::Pong,
            IpcCommand::Stop(_) => IpcResponse::Ok,
            IpcCommand::StopAll => IpcResponse::OkCount(1),
        };

        tx.send(cmd)
            .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()))?;

        let mut writer = stream;
        writer.write_all(format!("{}\n", response.to_string()).as_bytes())?;
        writer.flush()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parse() {
        assert_eq!(IpcCommand::parse("PING"), Some(IpcCommand::Ping));
        assert_eq!(IpcCommand::parse("STOP:0"), Some(IpcCommand::Stop(0)));
        assert_eq!(IpcCommand::parse("STOP:1"), Some(IpcCommand::Stop(1)));
        assert_eq!(IpcCommand::parse("STOP:ALL"), Some(IpcCommand::StopAll));
        assert_eq!(IpcCommand::parse("INVALID"), None);
    }

    #[test]
    fn test_command_to_string() {
        assert_eq!(IpcCommand::Ping.to_string(), "PING");
        assert_eq!(IpcCommand::Stop(0).to_string(), "STOP:0");
        assert_eq!(IpcCommand::StopAll.to_string(), "STOP:ALL");
    }

    #[test]
    fn test_response_parse() {
        assert_eq!(IpcResponse::parse("OK"), Some(IpcResponse::Ok));
        assert_eq!(IpcResponse::parse("OK:3"), Some(IpcResponse::OkCount(3)));
        assert_eq!(IpcResponse::parse("PONG"), Some(IpcResponse::Pong));
        assert_eq!(
            IpcResponse::parse("ERR:test error"),
            Some(IpcResponse::Error("test error".to_string()))
        );
    }

    #[test]
    fn test_response_to_string() {
        assert_eq!(IpcResponse::Ok.to_string(), "OK");
        assert_eq!(IpcResponse::OkCount(5).to_string(), "OK:5");
        assert_eq!(IpcResponse::Pong.to_string(), "PONG");
        assert_eq!(
            IpcResponse::Error("failed".to_string()).to_string(),
            "ERR:failed"
        );
    }
}
