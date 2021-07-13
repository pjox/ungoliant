//! Command line arguments and parameters management/parsing.
use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "oscar-tools", about = "A collection of tools for OSCAR corpus")]
/// Holds every command that is callable by the `oscar-tools` command.
pub enum Ungoliant {
    #[structopt(about = "Downloading of CommonCrawl")]
    Download(Download),
    #[structopt(about = "Run pipeline")]
    Pipeline(Pipeline),
}

#[derive(Debug, StructOpt)]
/// Download command and parameters.
/// ```sh
/// ungoliant-download 0.1.0
/// Downloading of CommonCrawl
///
/// USAGE:
///     ungoliant download [OPTIONS] <paths-file> <dst>
///
/// FLAGS:
///     -h, --help       Prints help information
///     -V, --version    Prints version information
///
/// OPTIONS:
///     -t <n-tasks>        number of tokio tasks. Default is 4.
///     -o <offset>         number of files to skip. Default is 0.
///
/// ARGS:
///     <paths-file>    path to wet.paths file
///     <dst>           download destination
/// ```
pub struct Download {
    #[structopt(parse(from_os_str), help = "path to wet.paths file")]
    pub paths_file: PathBuf,
    #[structopt(parse(from_os_str), help = "download destination")]
    pub dst: PathBuf,
    #[structopt(short = "t", help = "number of tokio tasks. Default is 4.")]
    pub n_tasks: Option<usize>,
    #[structopt(short = "o", help = "number of files to skip. Default is 0.")]
    pub offset: Option<usize>,
}

#[derive(Debug, StructOpt)]
/// Pipeline command and parameters.
///
/// ```sh
/// ungoliant-pipeline 0.1.0
/// Run pipeline
///
/// USAGE:
///     ungoliant pipeline [FLAGS] <src> <dst>
///
/// FLAGS:
///     -h, --help             Prints help information
///     -V, --version          Prints version information
///     -m, --with_metadata    extract metadata
///
/// ARGS:
///     <src>    source (contains n.txt.gz)
///     <dst>    pipeline result destination
/// ```
pub struct Pipeline {
    #[structopt(parse(from_os_str), help = "source (contains n.txt.gz)")]
    pub src: PathBuf,
    #[structopt(parse(from_os_str), help = "pipeline result destination")]
    pub dst: PathBuf,
    #[structopt(
        parse(from_os_str),
        long = "lid-path",
        help = "Path to 176.lid.bin",
        default_value = "lid.176.bin"
    )]
    pub lid_path: PathBuf,
    #[structopt(
        short = "s",
        long = "part_size",
        help = "maximum part size in MBytes",
        default_value = "500"
    )]
    pub part_size: u64,
}
