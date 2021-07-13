//! # Ungoliant
//!
//! 🕷️ Ungoliant is the upcoming pipeline to obtain an OSCAR corpus from a Common Crawl dump.
//! This pipeline replaces the original goclassy pipeline.
//!
//! This project can be used both as a tool to download or generate corpora,
//! or as a lib to integrate downloading and processing into other projects.
//!
//! ## Getting started
//!
//! ```sh
//! oscar-tools 0.1.0
//! A collection of tools for OSCAR corpus
//!
//! USAGE:
//!     ungoliant <SUBCOMMAND>
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! SUBCOMMANDS:
//!     download    Downloading of CommonCrawl
//!     help        Prints this message or the help of the given subcommand(s)
//!     pipeline    Run pipeline
//! ```
//!

use download::Downloader;
use std::fs::File;
use std::io::Write;
use structopt::StructOpt;

#[macro_use]
extern crate log;

mod classify;
mod cli;
mod download;
mod error;
mod lang;
mod pipeline;
mod shard;
mod writing;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    env_logger::init();

    let opt = cli::Ungoliant::from_args();
    debug!("cli args\n{:#?}", opt);

    match opt {
        cli::Ungoliant::Download(e) => {
            let paths = File::open(e.paths_file)?;
            let mut dl = Downloader::from_paths_file(&paths, e.n_tasks.unwrap_or(4))?;
            let results = dl.download(&e.dst, e.offset).await;

            let mut error_file = File::create("errors.txt")?;

            // write eventual download errors
            for failure in results.iter().filter(|result| result.is_err()) {
                error!("Error during download:\n {:?}", failure);
                // match failure.as_ref().unwrap_err() {
                //     download::Error::Download(e) => {
                //         write!(error_file, "{}\t{}", e.err.url().unwrap(), e.id)?;
                //     }
                //     _ => (),
                // };
                if let download::Error::Download(e) = failure.as_ref().unwrap_err() {
                    write!(error_file, "{}\t{}", e.err.url().unwrap(), e.id)?;
                }
            }
        }

        cli::Ungoliant::Pipeline(p) => {
            let p = pipeline::OscarMetadata::new(p.src, p.dst, p.lid_path, p.part_size);
            p.run()?;
        }
    };
    Ok(())
}
