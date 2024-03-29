#![allow(dead_code)]

use crate::analysis::analysis_command::ExecuteAnalysisCommandHandler;
use crate::analyzer_event::analyzer_event_subsciber_stdout::AnalyzerEventSubscriverStout;
use crate::analyzer_event::analyzer_event_subscriber_analysis::AnalyzerEventSubscriberAnalysis;
use crate::analyzer_event::analyzer_event_subscriber_udp_metric::AnalyzerEventSubscriberUdpMetric;
use crate::broadcast::BroadcastEvent;
use crate::http::http_analyzer::HttpAnalyzer;
use crate::http::http_check::HttpCheck;
use mehsh_common::config::Config;
use std::path::PathBuf;
use clap::Parser;
use tokio::runtime::{Builder, Runtime};
use udp_echo::analyzer::Analyzer;
use udp_echo::client::Client;
use udp_echo::server::Server;

pub mod analysis;
pub mod analyzer_event;
pub mod broadcast;
pub mod http;
pub mod udp_echo;
pub mod maintenance_mode;

#[macro_use]
extern crate mehsh_common;

#[derive(Parser, Debug)]
struct Opt {
    /// Files to process
    #[arg(name = "config")]
    config: PathBuf,

    #[arg(long = "name", default_value = "[hostname]")]
    name: String,
    /*
    #[structopt(long = "privatekey")]
    privatekey: String,
    */
}

fn main() {
    let opt = Opt::parse();
    println!("opt: {:#?}", &opt);

    let rt: Runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("could not build runtime");

    match try_main(opt, rt) {
        Err(err) => {
            eprintln!("{:?}", &err);

            for cause in err.chain().into_iter() {
                println!("{:?}", cause);
            }
        }
        Ok(_) => {}
    }
}

fn try_main(opt: Opt, rt: Runtime) -> Result<(), ::anyhow::Error> {
    let name_self = opt.name.replace(
        "[hostname]",
        hostname::get()
            .expect("Hostname should be a string!")
            .into_string()
            .expect("Hostname should be a string!")
            .as_str(),
    );

    let config = Config::new_from_file(name_self.clone(), opt.config)?;

    println!("{:#?}", &config);

    let (broardcast_sender, broardcast_recv) =
        ::tokio::sync::broadcast::channel::<BroadcastEvent>(1000);

    if config.get_server_self().serverdensity_udp_agent {
        let udp_boardcast_recv = broardcast_sender.subscribe();
        rt.spawn(async move {
            AnalyzerEventSubscriberUdpMetric::new(udp_boardcast_recv)
                .run()
                .await
        });
    }

    for analysis_entry in config.all_analyisis()?.into_iter() {
        if analysis_entry.from.identifier.to_string() != name_self.as_str() {
            continue;
        }

        if analysis_entry.to.identifier.to_string() == name_self.as_str() {
            continue;
        }

        println!(
            "preparing analysis {} from: {} to: {}",
            &analysis_entry.name, &analysis_entry.from.identifier, &analysis_entry.to.identifier
        );

        let udp_boardcast_recv = broardcast_sender.subscribe();

        rt.spawn(async move {
            AnalyzerEventSubscriberAnalysis::new(analysis_entry, udp_boardcast_recv)
                .run()
                .await;
        });
    }

    let udp_analyzer = Analyzer::new(config.clone());
    let udp_analyzer_sender = udp_analyzer.get_sender_handle();
    rt.spawn(async move { udp_analyzer.run(broardcast_sender).await });

    let http_analyzer = HttpAnalyzer::new(config.clone());
    let http_analyzer_sender = http_analyzer.get_sender_handle();
    rt.spawn(async move { http_analyzer.run().await });

    let handle = rt.spawn(async move { Server::new("0.0.0.0:4232").await?.run().await });

    for check in config.all_checks()?.into_iter() {
        if check.from.identifier.to_string() != name_self.as_str() {
            continue;
        }

        match check.check.as_str() {
            "udp_ping" => {
                let client_analyzer_sender = udp_analyzer_sender.clone();
                println!("starting check to {}", &check.to.identifier);
                rt.spawn(async move {
                    Client::new(check.clone(), client_analyzer_sender)
                        .await?
                        .run()
                        .await
                });
            }
            "http" => {
                let client_analyzer_sender = http_analyzer_sender.clone();
                rt.spawn(async move {
                    HttpCheck::new(check.clone(), client_analyzer_sender)
                        .run()
                        .await
                });
            }
            _ => {
                panic!("unknown check.");
            }
        }
    }

    rt.spawn(async move {
        AnalyzerEventSubscriverStout::new(broardcast_recv)
            .run()
            .await
    });

    rt.block_on(handle)
        .expect("could not block on handle")
        .expect("could not block on handle#2");

    Ok(())
}
