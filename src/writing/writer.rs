/*! Text&Metadata writer for a given language.

Holds writing and rotating on both text and metadata files for a given language.
Supports writing of numerous [MergedPiece], given that their identification are the same.
Identification is checked too, preventing the writing of differently identified [MergedPiece] into a given language writer.
!*/
use std::convert::TryFrom;
use std::io::Write;
use std::path::Path;

use crate::pipeline::Metadata;
use log::{debug, error};

use crate::pipeline::oscar_metadata::document::{MergedPiece, PartChunk};
use crate::{
    error,
    writing::{MetaWriter, TextWriter},
};

pub struct Writer {
    handle_text: TextWriter,
    handle_meta: MetaWriter,
    lang: &'static str,
    offset: usize,
}

impl Writer {
    /// Create a new Writer for provided language.
    /// Files will be written at the root of the `dst` file, and shouldn't exceed `size_limit`.
    ///
    /// _See [TextWriter] to have an explanation about the *shouldn't*._
    pub fn new(dst: &Path, lang: &'static str, size_limit: u64) -> Result<Self, error::Error> {
        Ok(Self {
            handle_text: TextWriter::new(dst, lang, size_limit),
            handle_meta: MetaWriter::new(dst, lang),
            lang,
            offset: 0,
        })
    }

    /// writes the provided [MergedPiece], checking language identification.
    pub fn write(&mut self, pieces: Vec<MergedPiece>) -> Result<(), error::Error> {
        // get size of whole pieces.
        // If all the pieces fit, we bulk insert.
        let whole_size =
            u64::try_from(pieces.iter().fold(0, |acc, x| acc + x.sentences.len())).unwrap();

        if whole_size < self.handle_text.get_free_space() {
            debug!("writing whole chunk.");
            debug!("current offset is {}", self.offset);
            let mut pc = PartChunk::new(pieces)?;
            debug!(
                "partchunk last offset is {} ({} with nb_sentences)",
                pc.metadata.last().unwrap().offset,
                pc.metadata.last().unwrap().offset + pc.metadata.last().unwrap().nb_sentences
            );
            if let Some(new_offset) = pc.bump_offsets(self.offset) {
                debug!(
                    "partchunk bumped last offset is {} ({} with nb_sentences)",
                    pc.metadata.last().unwrap().offset,
                    pc.metadata.last().unwrap().offset + pc.metadata.last().unwrap().nb_sentences
                );
                self.offset = new_offset;
                debug!("next lines will have base offset at {}", self.offset);
            } else {
                error!("no new offset?");
            }

            self.handle_text.write_all(&pc.body.as_bytes())?;
            // println!(
            //     "{}: offset of the last metadata: {:#?}",
            //     self.lang,
            //     pc.metadata.last().unwrap().offset
            // );
            let mut metadata = serde_json::to_string_pretty(&pc.metadata).unwrap(); //todo add from error
            metadata.pop();
            metadata.push(',');
            let metadata: &str = &metadata[1..metadata.len()];
            self.handle_meta.write_all(&metadata.as_bytes())?;
        } else {
            for piece in pieces {
                //ensure that the piece has the correct language identification
                self.write_single(&piece)?;
            }
        }

        Ok(())
    }

    fn write_single(&mut self, piece: &MergedPiece) -> Result<(), error::Error> {
        if piece.identification() != self.lang {
            return Err(error::Error::Custom(format!(
                "Wrong language. Tried to add a {} piece into a {} file.",
                piece.identification(),
                self.lang
            )));
        }

        self.handle_text.write_all(piece.sentences.as_bytes())?;
        // trigger new file creation for metadata if applicable
        // reset offest
        if self.handle_text.first_write_on_document {
            // ignore if <= 1 since it's the first file
            if self.handle_text.nb_files > 1 {
                self.handle_meta.create_next_file()?;
                self.offset = 0;
            }
            self.handle_text.first_write_on_document = false;
        }

        let mut metadata = Metadata::try_from(piece.headers.clone())?;

        // update defaulted values in metadata
        metadata.nb_sentences = piece.nb_sentences;
        metadata.offset = self.offset;

        // update lang offset
        self.offset += metadata.nb_sentences + 1;

        let mut metadata_str = serde_json::to_string_pretty(&metadata).unwrap(); //todo add from for error
        metadata_str.push(',');

        self.handle_meta.write_all(metadata_str.as_bytes())?;
        Ok(())
    }
    /// Binds to [MetaWriter::close_file].
    /// Closes current metadata file.
    pub fn close_meta(&mut self) -> Result<(), error::Error> {
        self.handle_meta.close_file()
    }
}
#[cfg(test)]
mod tests {

    use std::{collections::HashMap, fs::File, io::Read};

    use warc::header::WarcHeader;

    use super::*;

    type WarcHeaders = HashMap<WarcHeader, Vec<u8>>;

    #[test]
    fn test_init() {
        let dst = Path::new("dst_test_init_writer");
        std::fs::create_dir(dst).unwrap();
        let _ = Writer::new(dst, "en", 1_000_000);
        std::fs::remove_dir_all(dst).unwrap();
    }

    #[test]
    fn write() {
        let dst = Path::new("dst_test_write");
        std::fs::create_dir(dst).unwrap();
        let mut wr = Writer::new(dst, "fr", 10).unwrap();

        let headers: WarcHeaders =
            vec![(WarcHeader::Filename, Vec::from("filenametest".as_bytes()))]
                .into_iter()
                .collect();
        let merged_pieces = vec![MergedPiece {
            sentences: "Bonjour, c'est moi!
Comment allez-vous?
Bien, et vous?
Ecoutez ça va plutôt bien."
                .to_string(),
            nb_sentences: 4,
            identification: "fr",
            headers,
        }];

        wr.write(merged_pieces.to_vec()).unwrap();
        wr.close_meta().unwrap();

        // check if content is the same
        let mut sentences = String::new();
        let mut f = File::open("dst_test_write/fr.txt").unwrap();
        f.read_to_string(&mut sentences).unwrap();

        //to account for \n\n
        let mut from_merged_pieces = merged_pieces[0].sentences.clone();
        from_merged_pieces.push_str("\n\n");

        assert_eq!(sentences, from_merged_pieces);

        // succintly check if metadata are the same
        let f = File::open("dst_test_write/fr_meta.json").unwrap();
        let metadata: Vec<Metadata> = serde_json::from_reader(f).unwrap();
        assert_eq!(metadata[0].nb_sentences, merged_pieces[0].nb_sentences);
        std::fs::remove_dir_all(dst).unwrap();
    }

    #[test]
    fn write_multiple() {
        let dst = Path::new("dst_test_write_multiple");
        std::fs::create_dir(dst).unwrap();
        let mut wr = Writer::new(dst, "fr", 10_000).unwrap();

        let mut merged_pieces = Vec::new();
        for i in 1..10 {
            let headers: WarcHeaders = vec![(
                WarcHeader::Filename,
                Vec::from(format!("filenametest{}", i).as_bytes()),
            )]
            .into_iter()
            .collect();

            let sentences = vec!["lorem ipsum".to_string(); i].join("\n");
            let nb_sentences = i;
            let identification = "fr";

            merged_pieces.push(MergedPiece {
                sentences,
                headers,
                nb_sentences,
                identification,
            });
        }

        wr.write(merged_pieces.to_vec()).unwrap();
        wr.close_meta().unwrap();

        // check if content is the same
        let mut sentences = String::new();
        let mut f = File::open("dst_test_write_multiple/fr.txt").unwrap();
        f.read_to_string(&mut sentences).unwrap();
        let sentences: Vec<&str> = sentences.split("\n\n").collect();
        for i in 0..merged_pieces.len() {
            assert_eq!(sentences[i], merged_pieces[i].sentences);
        }

        // succintly check if metadata are the same
        let f = File::open("dst_test_write_multiple/fr_meta.json").unwrap();
        let metadata: Vec<Metadata> = serde_json::from_reader(f).unwrap();
        assert_eq!(metadata[0].nb_sentences, merged_pieces[0].nb_sentences);
        std::fs::remove_dir_all(dst).unwrap();
    }
}
