use std::process::{ExitStatus, Output, Stdio};
use anyhow::{anyhow, Context};
use futures::channel;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use mehsh_common::config::ConfigAnalysis;
use crate::BroadcastEvent;

type CliOutput = Vec<u8>;

pub struct ExecuteAnalysisCommandHandler {
    config_analysis: ConfigAnalysis,
    notify_send: UnboundedSender<()>,
    notify_recv: UnboundedReceiver<()>
}

impl ExecuteAnalysisCommandHandler {

    pub async fn new(config_analysis : ConfigAnalysis) -> Self {
        let (notify_send, notify_recv) = unbounded_channel::<()>();

        Self {
            config_analysis,
            notify_send,
            notify_recv
        }
    }

    pub async fn run_if_not_running(&self) {
        self.notify_send.send(());
    }

    async fn execute(&mut self, config : &ConfigAnalysis) {
        let (sender, mut receiver) = ::tokio::sync::mpsc::unbounded_channel::<CliOutput>();
        let (notify_command_finished, mut notify_receiver_finished) = unbounded_channel::<()>();

        let mut jh = None;

        loop {
            ::tokio::select! {
                // called when we need to run the command.
                notify = self.notify_recv.recv() => {
                    if notify.is_none() {
                        continue;
                    }

                    if jh.is_some() {
                        println!("command already running");
                        continue;
                    }


                    let execute_config = config.clone();
                    let execute_sender = sender.clone();
                    let notify_command_finished = notify_command_finished.clone();
                    jh = Some(::tokio::spawn(async move {
                        execute_analysis_command(&execute_config, execute_sender).await;
                        notify_command_finished.send(()).expect("could not send notify_command_finished");
                    }));

                    format!("started command.");
                },
                // command finished
                res = notify_receiver_finished.recv() => {
                    if res.is_none() {
                        continue;
                    }

                    jh.as_mut().unwrap().abort(); // it must be already abortet.
                    jh = None;

                    println!("finished command");

                    format!("finished command, exit");
                },
                res = receiver.recv() => {
                    let res = match res {
                        Some(s) => s,
                        None => continue,
                    };

                    println!("command output: {}", String::from_utf8_lossy(&res));
                }
            };
        }


        // jh.abort();

    }
}


pub async fn execute_analysis_command(config : &ConfigAnalysis, sender: UnboundedSender<CliOutput>) -> Result<ExitStatus, ::anyhow::Error> {
    let mut command = Command::new("/bin/bash");
    let command_with_args = command
        .args(&["-c", &config.command]);

    let mut child = command_with_args
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
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
                let out : &[u8] = &stdout_buffer[..stdout_read_res.context("could not read stdout_read_res")?];
                sender.send(out.to_vec())?;
            },
            stderr_read_res = stderr.read(&mut stderr_buffer) => {
                let out : &[u8] = &stderr_buffer[..stderr_read_res.context("could not read stdout_read_res")?];
                sender.send(out.to_vec())?;
            },
            res = child.wait() => {
                return res.map_err(|e| anyhow!(e));
            },
        }
    }
}