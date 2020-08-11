use clap;
use env_logger;
use rft::*;

#[macro_use]
mod common;
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

    // Parse command line options and call run methods:

    let opt;
    match options::Options::parse(
        matches.value_of("t"),
        matches.value_of("p"),
        matches.value_of("q"),
    ) {
        Err(e) => {
            eprintln!("Error while parsing command line options: {}", e);
            std::process::exit(1);
        }
        Ok(o) => opt = o,
    }

    if matches.is_present("s") {
        // server mode
        std::process::exit(match run_server(opt) {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("{:?}", e);
                1
            }
        });
    } else {
        // client mode

        let socket_addr;
        match options::parse_host(matches.value_of("host").unwrap()) {
            // unwrap() used since clap arg constraints should ensure that a host is present
            Err(e) => {
                eprintln!("Error while parsing command line options (host): {}", e);
                std::process::exit(1);
            }
            Ok(o) => socket_addr = o,
        }

        if matches.is_present("list") {
            // file list client
            let directory = matches.value_of("list").unwrap(); // unwrap() used since clap arg constraints should ensure that a directory is present

            std::process::exit(match run_ls_client(opt, socket_addr, directory) {
                Ok(_) => 0,
                Err(e) => {
                    eprintln!("{:?}", e);
                    1
                }
            });
        } else {
            // regular client
            let files = matches.values_of("file").unwrap().collect(); // unwrap() used since clap arg constraints should ensure that files are present

            std::process::exit(match run_client(opt, socket_addr, files) {
                Ok(_) => 0,
                Err(e) => {
                    eprintln!("{:?}", e);
                    1
                }
            });
        }
    }
}
