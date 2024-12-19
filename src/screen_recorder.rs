use rustix::process;
use rustix::process::Pid;
use std::io;
use std::process::Stdio;
use tokio::process::{Child, Command};

pub struct ScreenRecorder {
    child: Child,
}

impl ScreenRecorder {
    pub fn start() -> io::Result<Self> {
        let mut command = Command::new("wf-recorder");

        // Include audio.
        command.arg("-a");

        // 10 frames per second at most.
        command.arg("-r").arg(10_u16.to_string());

        // Output to `/tmp/unknown.mkv`.
        command.arg("-f").arg("/tmp/unknown.mkv");

        // Overwrite existing output file if present.
        command.arg("-y");

        // Redirect all output to `/dev/null`.
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        // Just in case.
        command.kill_on_drop(true);

        command.spawn().map(|child| Self { child })
    }

    pub async fn stop(mut self) -> io::Result<()> {
        let process_id = self
            .child
            .id()
            .and_then(|process_id| Pid::from_raw(process_id as i32))
            .ok_or_else(|| io::Error::other("unknown process id"))?;

        process::kill_process(process_id, rustix::process::Signal::Int)?;
        self.child.wait().await?;

        Ok(())
    }
}
