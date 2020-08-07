use clap::{App, AppSettings, Arg, SubCommand};

#[macro_use]
mod common;
mod transport;
mod client;
mod server;

fn main() {
    let matches = App::new("rft")
        .version("0.1.0")
        .about("RFT - a rusty Robust File Transfer protocol")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("server")
                .about("Start a server offering files for download.")
                .arg(
                    Arg::with_name("directory")
                        .index(1)
                        .help("Directory to serve files from.")
                        .takes_value(true)
                        .default_value("./"),
                ),
        )
        .subcommand(
            SubCommand::with_name("dl")
                .alias("download")
                .about("Download a file from a server.")
                .arg(
                    Arg::with_name("server")
                        .index(1)
                        .help("Server address to connect to.")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("file")
                        .index(2)
                        .help("The remote files path.")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("Local destination for the downloaded file.")
                        .takes_value(true),
                ),
        )
        .get_matches();

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
}
