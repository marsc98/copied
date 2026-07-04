use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use copied_core::{Command, Response};

pub struct IpcClient {
    stream: UnixStream,
}

impl IpcClient {
    pub fn connect() -> io::Result<Self> {
        let socket_path = socket_path()?;
        let stream = UnixStream::connect(&socket_path).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "failed to connect to daemon socket at {}: {err}",
                    socket_path.display()
                ),
            )
        })?;
        Ok(Self { stream })
    }

    pub fn send(&mut self, cmd: &Command) -> io::Result<Response> {
        let mut payload = serde_json::to_vec(cmd).map_err(io::Error::other)?;
        payload.push(b'\n');
        self.stream.write_all(&payload)?;
        self.stream.flush()?;

        let mut reader = BufReader::new(&self.stream);
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "daemon closed connection without sending a response",
            ));
        }

        serde_json::from_str(line.trim_end()).map_err(io::Error::other)
    }
}

fn socket_path() -> io::Result<PathBuf> {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "XDG_RUNTIME_DIR is not set; cannot locate daemon socket",
        )
    })?;
    Ok(PathBuf::from(runtime_dir).join("copied.sock"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn send_writes_command_line_and_reads_response_line() {
        let dir = tempfile_dir();
        let socket_path = dir.join("test.sock");
        let listener = UnixListener::bind(&socket_path).expect("bind listener");

        let server = std::thread::spawn(move || {
            let (server_stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(&server_stream);
            let mut line = String::new();
            reader.read_line(&mut line).expect("read command line");

            let cmd: Command = serde_json::from_str(line.trim_end()).expect("parse command");
            assert_eq!(cmd, Command::List);

            let response = Response::Ack;
            let mut payload = serde_json::to_vec(&response).expect("serialize response");
            payload.push(b'\n');
            (&server_stream)
                .write_all(&payload)
                .expect("write response");
        });

        let stream = UnixStream::connect(&socket_path).expect("connect");
        let mut client = IpcClient { stream };
        let response = client.send(&Command::List).expect("send");
        assert_eq!(response, Response::Ack);

        server.join().expect("server thread");
        let _ = std::fs::remove_file(&socket_path);
    }

    fn tempfile_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("copied-ipc-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
