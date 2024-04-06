use std::env::args;
use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;

pub fn usage() {
    print!(
        r###"Lemurs {}
{}
A TUI Display/Login Manager

USAGE: lemurs [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -c, --config <FILE>    A file to replace the default configuration
    -v, --variables <FILE> A file to replace the set variables
    -h, --help             Print help information
        --no-log
        --preview
        --tty <N>          Override the configured TTY number
        --xsessions <DIR>  Override the path to /usr/share/xsessions
        --wlsessions <DIR> Override the path to /usr/share/wayland-sessions
    -V, --version          Print version information

SUBCOMMANDS:
    cache
    envs
    help     Print this message or the help of the given subcommand(s)
"###,
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    );
}

pub struct Cli {
    pub preview: bool,
    pub no_log: bool,
    pub tty: Option<u8>,
    pub config: Option<PathBuf>,
    pub variables: Option<PathBuf>,
    pub command: Option<Commands>,
    pub xsessions: Option<PathBuf>,
    pub wlsessions: Option<PathBuf>,
}

pub enum Commands {
    Envs,
    Cache,
    Help,
    Version,
}

#[derive(Debug)]
pub enum CliError {
    MissingArgument(&'static str),
    InvalidTTY,
    InvalidArgument(String),
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::MissingArgument(flag) => {
                write!(f, "Missing an argument for the given flag '{flag}'")
            }
            CliError::InvalidTTY => {
                write!(f, "Given an invalid TTY number (only 1-12 are allowed)")
            }
            CliError::InvalidArgument(arg) => {
                write!(f, "Given an invalid flag or command '{arg}'")
            }
        }
    }
}

impl Error for CliError {}

impl Cli {
    pub fn parse() -> Result<Self, CliError> {
        let mut cli = Cli {
            preview: false,
            no_log: false,
            tty: None,
            config: None,
            variables: None,
            command: None,
            xsessions: None,
            wlsessions: None,
        };

        let mut args = args().skip(1).enumerate();
        while let Some((i, arg)) = args.next() {
            match (i, arg.trim()) {
                (0, "envs") => cli.command = Some(Commands::Envs),
                (0, "cache") => cli.command = Some(Commands::Cache),
                (0, "help") | (_, "--help") | (_, "-h") => cli.command = Some(Commands::Help),
                (_, "--version") | (_, "-V") => cli.command = Some(Commands::Version),

                (_, "--preview") => cli.preview = true,
                (_, "--no-log") => cli.no_log = true,
                (_, "--tty") => {
                    let (_, arg) = args.next().ok_or(CliError::MissingArgument("tty"))?;
                    let arg = arg.parse().map_err(|_| CliError::InvalidTTY)?;

                    if arg == 0 || arg > 12 {
                        return Err(CliError::InvalidTTY);
                    }

                    cli.tty = Some(arg);
                }
                (_, "--config") | (_, "-c") => {
                    let (_, arg) = args.next().ok_or(CliError::MissingArgument("config"))?;
                    let arg = PathBuf::from(arg);
                    cli.config = Some(arg);
                }
                (_, "--xsessions") => {
                    let (_, arg) = args.next().ok_or(CliError::MissingArgument("xsessions"))?;
                    let arg = PathBuf::from(arg);
                    cli.xsessions = Some(arg);
                }
                (_, "--wlsessions") => {
                    let (_, arg) = args.next().ok_or(CliError::MissingArgument("wlsessions"))?;
                    let arg = PathBuf::from(arg);
                    cli.wlsessions = Some(arg);
                }
                (_, "--variables") | (_, "-v") => {
                    let (_, arg) = args.next().ok_or(CliError::MissingArgument("variables"))?;
                    let arg = PathBuf::from(arg);
                    cli.variables = Some(arg);
                }
                (_, arg) => return Err(CliError::InvalidArgument(arg.to_string())),
            }
        }

        Ok(cli)
    }
}
