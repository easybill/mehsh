#![allow(dead_code)]

use std::path::PathBuf;
use structopt::StructOpt;
use failure::Error;
use mehsh_common::config::Config;
use tokio::runtime::{Runtime, Builder};
use udp_echo::server::Server;
use udp_echo::client::Client;
use udp_echo::analyzer::Analyzer;

pub mod udp_echo;

#[macro_use] extern crate failure;
extern crate mehsh_common;


#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    /// Files to process
    #[structopt(name = "config", parse(from_os_str))]
    config: PathBuf,

    #[structopt(long = "name")]
    name: String,

    /*
    #[structopt(long = "privatekey")]
    privatekey: String,
    */
}

fn main() {
    let opt = Opt::from_args();
    println!("opt: {:#?}", &opt);

    let rt : Runtime = Builder::new()
        .threaded_scheduler()
        .core_threads(4)
        .max_threads(10)
        .enable_all()
        .build()
        .unwrap();


    match try_main(opt, rt) {
        Err(err ) => {

            eprintln!("{:?}", &err);

            for cause in err.iter_causes() {
                println!("{:?}", cause);
            }
        },
        Ok(_) => {

        }
    }
}

fn try_main(opt : Opt, mut rt : Runtime) -> Result<(), Error> {

    let config = Config::new_from_file(opt.config)?;

    println!("{:#?}", &config);

    /*
    let idents = match config.resolve_idents(opt.name.clone()) {
        Err(_) => {
            eprintln!("could not resolve {}", &opt.name);
            panic!("nope");
        },
        Ok(k) => k
    };
    */

    let analyzer= Analyzer::new(config.clone());
    let analyzer_sender = analyzer.get_sender_handle();
    rt.spawn(async move {
        analyzer.run().await
    });

    let handle = rt.spawn(async move {
        Server::new("0.0.0.0:4232").await?.run().await
    });

    for check in config.all_checks()?.into_iter() {

        if check.from.identifier.to_string() != opt.name {
            continue;
        }


        match check.check.as_str() {
            "udp_ping" => {
                let client_analyzer_sender = analyzer_sender.clone();
                let remote = format!("{}:4232", check.to.ip.to_string());
                println!("starting check to {}", &remote);
                rt.spawn(async move {
                    Client::new(&remote, client_analyzer_sender).await?.run().await
                });
            }
            _ => {
                panic!("unknown check.");
            }
        }


    }

    rt.block_on(handle).expect("could not block on handle").expect("could not block on handle#2");

    Ok(())
}
