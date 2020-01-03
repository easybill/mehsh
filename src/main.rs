use std::path::PathBuf;
use structopt::StructOpt;
use failure::Error;
use crate::config::Config;
use tokio::runtime::{Runtime, Builder};
use crate::check::udp_echo::server::Server;
use crate::check::udp_echo::client::Client;
use std::thread::JoinHandle;
use crate::check::udp_echo::analyzer::Analyzer;

mod check;

#[macro_use] extern crate failure;


mod config;

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
    println!("opt: {:?}", &opt);

    let mut rt : Runtime = Builder::new()
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

    let mut analyzer= Analyzer::new(config.clone());
    let analyzer_sender = analyzer.get_sender_handle();
    rt.spawn(async move {
        analyzer.run().await
    });

    let handle = rt.spawn(async move {
        Server::new("0.0.0.0:4232").await?.run().await
    });

    let client_analyzer_sender = analyzer_sender;
    rt.spawn(async move {
        Client::new("127.0.0.1:4232", client_analyzer_sender).await?.run().await
    });

    rt.block_on(handle);

    Ok(())
}
