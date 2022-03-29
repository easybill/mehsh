use crate::BroadcastEvent;
use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use futures::channel;
use mehsh_common::config::ConfigAnalysis;
use std::process::{ExitStatus, Output, Stdio};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

type CliOutput = Vec<u8>;

pub struct ExecuteAnalysisCommandHandler {
    notify_send: UnboundedSender<()>,
}

struct CommandExecutionContext {
    jh: JoinHandle<()>,
    content: Vec<u8>,
    started: DateTime<Utc>,
}

#[derive(Debug)]
enum ExecuteMsg {
    CliOutput(CliOutput),
    Finish(()),
}

impl ExecuteAnalysisCommandHandler {
    pub fn new(config_analysis: ConfigAnalysis) -> Self {
        let (notify_send, notify_recv) = unbounded_channel::<()>();

        let s = Self { notify_send };

        ::tokio::spawn(async move {
            Self::execute(config_analysis, notify_recv).await;
        });

        s
    }

    pub fn run_if_not_running(&self) {
        self.notify_send
            .send(())
            .expect("could not notify ExecuteAnalysisCommandHandler");
    }

    async fn execute(config_analysis: ConfigAnalysis, mut notify_recv: UnboundedReceiver<()>) {
        let (execute_sender, mut execute_receiver) =
            ::tokio::sync::mpsc::unbounded_channel::<ExecuteMsg>();

        let mut command_execution_context = None;

        loop {
            ::tokio::select! {
                // called when we need to run the command.
                notify = notify_recv.recv() => {
                    if notify.is_none() {
                        continue;
                    }

                    if command_execution_context.is_some() {
                        // already running
                        continue;
                    }


                    let execute_config = config_analysis.clone();
                    let execute_sender = execute_sender.clone();
                    let jh = ::tokio::spawn(async move {
                        execute_analysis_command(&execute_config, execute_sender.clone()).await;
                        execute_sender.send(ExecuteMsg::Finish(())).expect("could not send notify_command_finished");
                    });

                    command_execution_context = Some(CommandExecutionContext {
                        jh,
                        content: vec![],
                        started: Utc::now(),
                    });

                    println!("started analysis {} from {} to {}.", &config_analysis.name, &config_analysis.from.identifier, &config_analysis.to.identifier);
                },
                res = execute_receiver.recv() => {
                    let mut res : ExecuteMsg = match res {
                        Some(s) => s,
                        None => continue,
                    };

                    match res {
                        ExecuteMsg::Finish(msg) => {
                            let context = match command_execution_context {
                                None => {
                                    println!("ERROR: command execution must exists");
                                    continue;
                                },
                                Some(ref mut c) => c,
                            };

                            context.jh.abort(); // it must be already aborted.


                            println!("finished analysis {}", &config_analysis.name);

                            match write_report_file(&config_analysis, &context).await {
                                Ok(filename) => {
                                    println!("wrote analysis {} report to {}", &config_analysis.name, filename);
                                },
                                Err(e) => {
                                    println!("Warning, could not write report {}", e);
                                }
                            };

                            command_execution_context = None;
                        },
                        ExecuteMsg::CliOutput(mut msg) => {
                            println!("analysis {} output: {}", &config_analysis.name, String::from_utf8_lossy(&msg));

                            match command_execution_context {
                                None => println!("warning, command execution context is empty. should never happen"),
                                Some(ref mut context) => {
                                    context.content.append(&mut msg)
                                }
                            };
                        }
                    };

                }
            };
        }
    }
}

async fn write_report_file(
    config_analysis: &ConfigAnalysis,
    command_execution_context: &CommandExecutionContext,
) -> Result<String, ::anyhow::Error> {
    let directory = format!(
        "/tmp/mehsh/{}/{}",
        &config_analysis.name, &config_analysis.to.identifier
    );

    ::tokio::fs::create_dir_all(&directory).await?;

    let end_date = Utc::now();
    let filename = format!(
        "{}/{}.txt",
        &directory,
        end_date.format("%Y_%m_%d_%H_%M_%S")
    );

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .append(true)
        .open(&filename)
        .await
        .context("could not create report file")?;

    file.write_all(command_execution_context.content.as_slice())
        .await
        .context("write command output")?;

    Ok(filename)
}

fn get_command_with_variables(config: &ConfigAnalysis) -> String {
    config
        .command
        .clone()
        .replace("{{server.from.ip}}", &config.from.ip.to_string())
        .replace("{{server.to.ip}}", &config.to.ip.to_string())
        .to_string()
}

async fn execute_analysis_command(
    config: &ConfigAnalysis,
    sender: UnboundedSender<ExecuteMsg>,
) -> Result<ExitStatus, ::anyhow::Error> {
    let mut command = Command::new("/bin/bash");
    let command_with_args = command.args(&["-c", &get_command_with_variables(config)]);

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
                if out.len() == 0 {
                    continue;
                }
                sender.send(ExecuteMsg::CliOutput(out.to_vec()))?;
            },
            stderr_read_res = stderr.read(&mut stderr_buffer) => {
                let out : &[u8] = &stderr_buffer[..stderr_read_res.context("could not read stdout_read_res")?];
                if out.len() == 0 {
                    continue;
                }
                sender.send(ExecuteMsg::CliOutput(out.to_vec()))?;
            },
            res = child.wait() => {
                return res.map_err(|e| anyhow!(e));
            },
        }
    }
}
