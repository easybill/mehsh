use std::path::PathBuf;
use structopt::StructOpt;
use failure::Error;
use crate::config::Config;

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


    match try_main(opt) {
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

fn try_main(opt : Opt) -> Result<(), Error> {

    let config = Config::new_from_file(opt.config)?;

    // println!("config: {:#?}", &config);

    let server_config = match config.get_server_clone(&opt.name) {
        Some(s) => s,
        None => {
            eprintln!(
                "could not find server, pick one of: {}",
                config.get_servers().iter().map(|m|{m.name.clone()}).collect::<Vec<_>>().join(", ")
            );
            return Ok(());
        }
    };

    Ok(())
}
