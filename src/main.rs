use clap;
use env_logger;

#[macro_use]
mod common;
mod client;
mod server;
mod transport;

extern crate num;
#[macro_use]
extern crate num_derive;

fn main() {
    // Initialize logger.
    env_logger::init();

    // Specify command line interface:
    let matches = clap::App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!("\n"))
        .about("RFT - a rust implementation of the SOFT protocol")
        .settings(&[clap::AppSettings::DeriveDisplayOrder, clap::AppSettings::StrictUtf8])
        // Options according to Lecture slides:
        .arg(clap::Arg::with_name("s")
            .short("s")
            .help("server mode: accept incoming files from any host\nOperate in client mode if “–s” is not specified")
            .required(true)
            .conflicts_with_all(&["host","file", "list"])
        )
        .arg(clap::Arg::with_name("host")
            .help("the host to send to or request from (hostname or IPv4 address)")
            .index(1)
            .required(true)
            .takes_value(true)
            .conflicts_with("s")
        )
        .arg(clap::Arg::with_name("t")
            .short("t")
            .help("specify the port number to use (use a default if not given)")
            .takes_value(true)
        )
        .arg(clap::Arg::with_name("p")
            .short("p")
            .help("'Packet n not Lost':\nspecify the loss probabilities for the Markov chain model\nif only one is specified, assume p=q; if neither is specified assume no\nloss")
            .takes_value(true)
        )
        .arg(clap::Arg::with_name("q")
            .short("q")
            .help("'Packet n lost':\nspecify the loss probabilities for the Markov chain model\nif only one is specified, assume p=q; if neither is specified assume no\nloss")
            .takes_value(true)
        )
        .arg(clap::Arg::with_name("file")
            .help("the name of the file(s) to fetch")
            .index(2)
            .multiple(true)
            .required(true)
            .takes_value(true)
            .conflicts_with_all(&["s", "list"])
        )
        // Other options:
        .arg(clap::Arg::with_name("list")
            .help("file list retrival")
            .short("l")
            .required(true)
            .takes_value(true)
            .conflicts_with_all(&["s", "file"])
        )
        .get_matches();

    /*
    if let Some(matches) = matches.subcommand_matches("dl") {
        let retcode = client::run(
            matches.value_of("server").unwrap(),
            matches.value_of("file").unwrap(),
            matches.value_of("output"),
        );
        std::process::exit(retcode);
    };
    if let Some(matches) = matches.subcommand_matches("server") {
        let retcode = server::run(matches.value_of("directory").unwrap());
        std::process::exit(retcode);
    };
    */
}
