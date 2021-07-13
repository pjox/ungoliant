//! Thread-safe language-separated text/metadata writer.
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::lang::LANG;
use crate::{error, writing::writer::Writer};
/// Holds references to [Writer].
pub struct LangFiles {
    writers: HashMap<&'static str, Arc<Mutex<Writer>>>,
}

impl LangFiles {
    /// Create a new LangFiles. `part_size_bytes` sets an indication of the maximum size
    /// by part.
    /// Note that if it is set too low and a unique record can't be stored in an unique part
    /// then a part will still be created, being larger than the `part_size_bytes`. This is expected behaviour.
    ///
    /// Also keep in mind that [Self::close_meta] has to be called once every write is done.
    ///
    // [Self::close_meta] could be integrated in an `impl Drop`
    pub fn new(dst: &Path, part_size_bytes: u64) -> Result<Self, error::Error> {
        let mut writers = HashMap::with_capacity(LANG.len());
        let mut w;
        for lang in LANG.iter() {
            w = Writer::new(dst, lang, part_size_bytes)?;
            writers.insert(*lang, Arc::new(Mutex::new(w)));
        }

        Ok(Self { writers })
    }

    /// Get a non-mutable reference to the writers.
    pub fn writers(&self) -> &HashMap<&'static str, Arc<Mutex<Writer>>> {
        &self.writers
    }

    /// Fix open metadata files by removing trailing comma and closing the array.
    pub fn close_meta(&self) -> Result<(), error::Error> {
        for writer in self.writers.values() {
            let mut writer_lock = writer.lock().unwrap();
            writer_lock.close_meta()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::pipeline::oscar_metadata::document::MergedPiece;
    use warc::header::WarcHeader;

    use super::*;

    type WarcHeaders = HashMap<WarcHeader, Vec<u8>>;

    fn create_merged_piece(
        sentences: String,
        identification: &'static str,
        headers: WarcHeaders,
    ) -> MergedPiece {
        let nb_sentences = sentences.split("\n").count();
        MergedPiece {
            sentences,
            identification,
            headers,
            nb_sentences,
        }
    }
    #[test]
    fn init() {
        let dst = Path::new("dst_langfiles_init");
        std::fs::create_dir(dst).unwrap();
        let _ = LangFiles::new(dst, 10);
        std::fs::remove_dir_all(dst).unwrap();
    }

    #[test]
    fn write_one() {
        let dst = Path::new("dst_langfiles_write_one");
        std::fs::create_dir(dst).unwrap();
        let langfiles = LangFiles::new(dst, 10).unwrap();

        let sentences = "essai d'écriture
de trois lignes
hehe :)"
            .to_string();
        let headers = vec![(WarcHeader::ContentType, Vec::from("blogpost".as_bytes()))]
            .into_iter()
            .collect();
        let mp = vec![create_merged_piece(sentences, "fr", headers)];
        // lock mutex and acquire writer
        let fr_writer = langfiles.writers().get("fr").unwrap().clone();
        let mut fr_writer_locked = fr_writer.lock().unwrap();

        fr_writer_locked.write(mp).unwrap();
        std::fs::remove_dir_all(dst).unwrap();
    }
}
