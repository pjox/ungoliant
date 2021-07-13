use std::{collections::HashMap, path::PathBuf};

use crate::lang::LANG;
use crate::shard::wet::Wet;
use crate::{classify::Classifier, pipeline::oscar_metadata::document::MergedPiece};
use crate::{error::Error, pipeline::oscar_metadata::document::Document};
use log::Level::Debug;
use log::{debug, error, info, log_enabled, warn};
use rayon::prelude::*;
use warc::{header::WarcHeader, RawRecord};

use crate::writing::LangFiles;
/// OSCAR v1.5 generation pipeline
///
/// OSCAR v1.5 is a retrocompatible corpus
/// enhanced with metadata coming from CommonCrawl.
///
/// The CommonCrawl dump is composed of shards,
/// Each shard is composed of records,
/// Each record is composed of a metadata header and a body containing sentences.
///
/// # Processing
/// _every scope is concurrent, that means green threads are created on shards, records and sentences._
/// - We process each record separately, getting a list of sentence-language pairs, along with metadata from the document.
/// - Once we've treated each record of a given shard, we
///   transform out list of sentence-language pairs into chunks of contiguous same-language sentences
///   and we store shard-level line offsets on metadata.
///   Then we group same-language chunks for each language (on shard-level) and we write on disk.
/// - We also keep track of disk-level line offsets to sync shard-level offsets between writes.
///
/// TODO: Better document this step.
pub struct OscarMetadata {
    src: PathBuf,
    dst: PathBuf,
    lid_path: PathBuf,
    part_size: u64,
}

/// convinience type alias for [warc::Record] headers.
type WarcHeaders = HashMap<WarcHeader, Vec<u8>>;

impl OscarMetadata {
    pub fn new(src: PathBuf, dst: PathBuf, lid_path: PathBuf, part_size: u64) -> Self {
        Self {
            src,
            dst,
            lid_path,
            part_size,
        }
    }

    /// attempt to predict language on provided sentence.
    ///
    /// Returns [None] if no language is detected.
    // why return the sentence itself?
    fn identify_sentence(sentence: &str, cls: &Classifier) -> Option<(String, &'static str)> {
        let prediction = cls.predict(&sentence).ok();

        if let Some(Some(lang)) = prediction {
            //TODO: rewrite these two lines more elegantly
            //      we can unwrap since predict returns None if no predictions are
            //      found
            let lang = lang.get(0).unwrap();

            // check if fasttext provided lang exists
            // return None if not
            match LANG.get(lang.label.as_str()) {
                Some(lang) => Some((sentence.to_string(), *lang)),
                None => {
                    warn!("lang {} does not exist!", lang.label);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Process a provided record.
    ///
    /// Here, sentences that are >100 chars are processed,
    /// and the others are discarded.
    /// See [String::chars::count].
    ///
    /// Then, we identify language for each sentence
    /// and return (sentence, language) along with headers
    /// extracted from the WARC.
    fn process_record(
        record: RawRecord,
        cls: &Classifier,
    ) -> Option<(Vec<(String, &'static str)>, WarcHeaders)> {
        if log_enabled!(Debug) {
            debug!(
                "processing record {}",
                String::from_utf8_lossy(
                    record
                        .headers
                        .get(&WarcHeader::RecordID)
                        .unwrap_or(&Vec::from("no record id".as_bytes()))
                )
            );
        };
        let body = String::from_utf8(record.body).ok();

        // process record if body is utf8-valid
        if let Some(sentences) = body {
            // filter out lines that does not contain 100 characters.
            // then convert into a parallel iterator
            let sentences = sentences
                .lines()
                .filter(|line| line.chars().count() > 100)
                .par_bridge();

            let results: Vec<(String, &'static str)> = sentences
                // predict for each sentence, discarding
                // predictions that does not meet threshold
                .filter_map(|sentence| Self::identify_sentence(sentence, cls))
                .collect();

            Some((results, record.headers))
        } else {
            error!(
                "body not UTF-8 valid: {:?}",
                record.headers.get(&WarcHeader::RecordID)
            );
            None
        }
    }

    /// Run the whole pipeline
    pub fn run(&self) -> Result<(), Error> {
        // let errors;

        let cls = Classifier::new(&self.lid_path, 1, 0.8)?;

        // list files in source folder,
        // filter out errors from fs and from gzip/wet.
        // This means that invalid gz files and invalid
        // wet files are discarded silently
        let results = std::fs::read_dir(&self.src)?
            .filter_map(|shard| {
                shard.map_or_else(
                    |e| {
                        error!("error reading shard directory: {}", e);
                        None
                    },
                    Some,
                )
            })
            .map(|shard| shard.path());

        // convert to parallel iterator
        // /!\: We use par_bridge, that is suboptimal
        //      compared to implementing IntoParallelIterator
        //      ourselves.
        let results = results.enumerate().par_bridge();

        // holds file handles
        let langfiles = LangFiles::new(&self.dst, self.part_size * 1_000_000)?;

        // iterate over shards
        let r: Vec<Error> = results
            .filter_map(|(idx, shard)| {
                // holds merged pieces by lang
                let mut lang_pieces: HashMap<&'static str, Vec<MergedPiece>> = HashMap::new();

                // get an atomic reference to global offsets
                // let offsets_global_arc = offsets_global.clone();
                info!("processing shard {}: {:?}", idx, &shard);

                let shard = Wet::from_path_gzip(&shard);
                if shard.is_err() {
                    error!("Could not read/open shard {}", idx);
                    return shard.err();
                }

                let shard = shard.unwrap();
                // convert into a parallel iterator
                let wetfile = shard.enumerate().par_bridge();

                let shard_results: Vec<(Vec<(String, &'static str)>, WarcHeaders)> = wetfile
                    .filter_map(|(idx_record, record)| match record {
                        Ok(record) => OscarMetadata::process_record(record, &cls),
                        Err(e) => {
                            warn!("Error on record {} of shard {}: {:?}", idx_record, idx, e);
                            None
                        }
                    })
                    // collect here is blocking
                    // because we can't write concurrently into a HashMap
                    // and using Mutexes might ruin performance.
                    .collect(); //TODO: test with a for_each and a channel to send?

                // Iterate over (record, header) tuples
                let shard_results = shard_results.into_iter().filter_map(|(record, header)| {
                    // split between langs and sentences
                    let langs: Vec<&str> = record.iter().map(|(_, lang)| *lang).collect();
                    let sentences: Vec<String> =
                        record.into_iter().map(|(sentences, _)| sentences).collect();

                    // create new document for current record
                    let doc = Document::new(header, sentences, langs);

                    match doc {
                        Ok(doc) => Some(doc),
                        Err(e) => {
                            warn!("{:?}", e);
                            None
                        }
                    }
                });

                // merge all documents together
                // get a vector of merged pieces of difference languages
                let docs_merged = shard_results
                    .map(|doc| doc.into_merged_pieces_lang())
                    .flatten()
                    .collect::<Vec<MergedPiece>>();

                // sort merged pieces into different langs
                // now there's a hashmap that points each lang
                // to a vector of merged pieces
                for piece in docs_merged {
                    let e = lang_pieces
                        .entry(piece.identification())
                        .or_insert_with(Vec::new);
                    e.push(piece);
                }

                // write concurrently
                lang_pieces.into_par_iter().for_each(|(lang, pieces)| {
                    let writer = langfiles.writers().get(lang).unwrap();
                    let mut writer_lock = writer.lock().unwrap();
                    writer_lock.write(pieces).unwrap();
                });

                None
            })
            .collect();

        // fix trailing comma
        langfiles.close_meta()?;

        for err in r {
            error!("{:?}", err);
        }

        Ok(())
    }
}
