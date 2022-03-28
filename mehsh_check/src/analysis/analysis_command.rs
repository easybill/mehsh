use std::process::Stdio;
use anyhow::Context;
use futures::io::BufReader;
use tokio::process::Command;
use mehsh_common::config::ConfigAnalysis;



pub async fn execute_analysis_command(config : &ConfigAnalysis) {
    let command = Command::new("/bin/bash")
        .args(&["-c", &config.command]);

    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false)
        .spawn()
        .context("could not start child")?;

    let mut stdout_buffer = [0; 4096];
    let mut stdout = BufReader::new(
        child
            .stdout
            .take()
            .context("could not take stdout from child")?,
    );
    let mut stderr_buffer = [0; 4096];
    let mut stderr = BufReader::new(
        child
            .stderr
            .take()
            .context("could not take stderr from child")?,
    );

    loop {
        ::tokio::select! {
            stdout_read_res = stdout.read(&mut stdout_buffer) => {
                self.on_stdout_read_res(&mut stdout_buffer, stdout_read_res).await?;
            },
            stderr_read_res = stderr.read(&mut stderr_buffer) => {
                self.on_stderr_read_res(&mut stderr_buffer, stderr_read_res).await?;
            },
            res = child.wait() => {
                return Ok(self.on_child_wait(res).await?);
            },
        }
    }
}