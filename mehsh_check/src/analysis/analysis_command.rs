
use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};

use mehsh_common::config::ConfigAnalysis;
use std::process::{ExitStatus, Stdio};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

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
    CliOutput(Vec<u8>),
    Finish(()),
}

impl ExecuteAnalysisCommandHandler {
    pub fn new(config_analysis: ConfigAnalysis) -> Self {
        let (notify_send, mut notify_recv) = unbounded_channel::<()>();

        let s = Self { notify_send };

        ::tokio::spawn(async move {
            loop {
                match Self::execute(config_analysis.clone(), &mut notify_recv).await {
                    Ok(_) => { println!("WARNING, ExecuteAnalysisCommandHandler::execute finished, should never happen"); },
                    Err(e) => { println!("WARNING, ExecuteAnalysisCommandHandler::execute finished with error, should never happen: {}", e); },
                }
            }
        });

        s
    }

    pub fn run_if_not_running(&self) {
        self.notify_send
            .send(())
            .expect("could not notify ExecuteAnalysisCommandHandler");
    }

    async fn execute(config_analysis: ConfigAnalysis, notify_recv: &mut UnboundedReceiver<()>) -> Result<(), ::anyhow::Error> {
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
                        match execute_analysis_command(&execute_config, execute_sender.clone()).await {
                            Ok(exit_status) if exit_status.success() => {},
                            Ok(exit_status) => {
                                println!("analysis tool failed with exit code {:?}", exit_status.code());
                            },
                            Err(e) => println!("WARNING, could not execute analysis command {}", e),
                        };
                        execute_sender.send(ExecuteMsg::Finish(())).expect("could not send notify_command_finished");
                    });

                    command_execution_context = Some(CommandExecutionContext {
                        jh,
                        content: vec![],
                        started: Utc::now(),
                    });

                    println!("analysis {} started - from {} to {}.", &config_analysis.name, &config_analysis.from.identifier, &config_analysis.to.identifier);
                },
                res = execute_receiver.recv() => {
                    let res : ExecuteMsg = match res {
                        Some(s) => s,
                        None => continue,
                    };

                    match res {
                        ExecuteMsg::Finish(_msg) => {
                            let context = match command_execution_context {
                                None => {
                                    println!("WARNING: command execution must exists");
                                    continue;
                                },
                                Some(ref mut c) => c,
                            };

                            context.jh.abort(); // it must be already aborted.


                            println!("analysis {} finished", &config_analysis.name);

                            match write_report_file(&config_analysis, &context).await {
                                Ok(filename) => {
                                    println!("analysis {} - wrote report to {}", &config_analysis.name, filename);
                                },
                                Err(e) => {
                                    println!("WARNING, could not write report {}", e);
                                }
                            };

                            command_execution_context = None;
                        },
                        ExecuteMsg::CliOutput(mut msg) => {
                            println!("analysis {} - output: {}", &config_analysis.name, String::from_utf8_lossy(&msg));

                            match command_execution_context {
                                None => println!("WARNING, command execution context is empty. should never happen"),
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
        .replace("{{server.from.extra1}}", &config.from.extra1.clone().unwrap_or("".to_string()))
        .replace("{{server.from.extra2}}", &config.from.extra2.clone().unwrap_or("".to_string()))
        .replace("{{server.from.extra3}}", &config.from.extra3.clone().unwrap_or("".to_string()))
        .replace("{{server.to.ip}}", &config.to.ip.to_string())
        .replace("{{server.to.extra1}}", &config.to.extra1.clone().unwrap_or("".to_string()))
        .replace("{{server.to.extra2}}", &config.to.extra2.clone().unwrap_or("".to_string()))
        .replace("{{server.to.extra3}}", &config.to.extra3.clone().unwrap_or("".to_string()))
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
        .expect("could not start child");

    let stdout = BufReader::new(
        child
            .stdout
            .take()
            .expect("could not take stdout from child"),
    );
    let stderr = BufReader::new(
        child
            .stderr
            .take()
            .expect("could not take stderr from child"),
    );

    let jh_sender = sender.clone();
    let mut jh_read = stdout;
    let jh_stdout : JoinHandle<Result<(), ::anyhow::Error>> = ::tokio::spawn(async move {
        let mut buffer = [0; 4096];
        loop {
            match jh_read.read(&mut buffer).await? {
                0 => return Ok(()),
                size => {
                    let out : &[u8] = &buffer[..size];
                    jh_sender.send(ExecuteMsg::CliOutput(out.to_vec())).context("could not send")?;
                },
            }
        }
    });

    let jh_sender = sender.clone();
    let mut jh_read = stderr;
    let jh_stderr : JoinHandle<Result<(), ::anyhow::Error>> = ::tokio::spawn(async move {
        let mut buffer = [0; 4096];
        loop {
            match jh_read.read(&mut buffer).await? {
                0 => return Ok(()),
                size => {
                    let out : &[u8] = &buffer[..size];
                    jh_sender.send(ExecuteMsg::CliOutput(out.to_vec())).context("could not send")?;
                },
            }
        }
    });

    let (jh1, jh2) = ::tokio::join!(jh_stdout, jh_stderr);
    jh1??;
    jh2??;

    return child.wait().await.map_err(|e| anyhow!(e));
}
