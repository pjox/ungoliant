//! Rotating file writers for text and metadata.
use log::{debug, error, info};
use std::convert::TryFrom;
use std::fs::OpenOptions;
use std::path::Path;
use std::{fs::File, io::Write, path::PathBuf};
/// Rotating file writers.
///
/// Implement [std::io::Write] and holds a size (bytes) limit.
///
/// Note: if a slice to write is larger than the whole limit, then it is an expected behaviour that
/// the size limit is ignored and a file is created.
pub struct TextWriter {
    lang: &'static str,
    dst: PathBuf,
    text: Option<File>,
    size: u64,
    size_limit: u64,
    pub nb_files: u64,
    pub first_write_on_document: bool,
}

impl TextWriter {
    /// Create a new [TextWriter].
    /// Note that nothing is created/written unless a write is performed.
    /// size_limit is in bytes.
    pub fn new(dst: &Path, lang: &'static str, size_limit: u64) -> Self {
        Self {
            lang,
            dst: dst.to_path_buf(),
            text: None,
            size: 0,
            size_limit,
            nb_files: 0,
            first_write_on_document: false,
        }
    }

    /// Rotate file.
    ///
    /// The first file is named `lang.txt`, and is renamed `lang_part_1.txt` if there's > 1 number of files.
    pub fn create_next_file(&mut self) -> std::io::Result<()> {
        let filename = if self.nb_files == 0 {
            format!("{}.txt", self.lang)
        } else {
            format!("{}_part_{}.txt", self.lang, self.nb_files + 1)
        };

        let mut path = self.dst.clone();
        path.push(filename);

        let mut options = OpenOptions::new();
        options.read(true).append(true).create(true);

        info!("creating {:?}", path);
        let text = options.open(path)?;

        // if nb_files == 1, rename lang.txt into lang_part_1.txt
        if self.nb_files == 1 {
            let mut from = self.dst.clone();
            from.push(format!("{}.txt", self.lang));
            let mut to = self.dst.clone();
            to.push(format!("{}_part_1.txt", self.lang));

            debug!("renaming {:?} to {:?}", from, to);
            std::fs::rename(from, to)?;
        }

        self.text = Some(text);

        self.size = 0;
        self.nb_files += 1;
        self.first_write_on_document = true;
        Ok(())
    }

    /// gets first_write_on_document and resets it to false.
    /// useful to check variable value, and to reset it to its default one
    // allow dead code if we decide to switch on it
    #[allow(dead_code)]
    pub fn get_reset_first_write(&mut self) -> bool {
        let ret = self.first_write_on_document;
        self.first_write_on_document = false;
        ret
    }

    /// returns remaining size in file
    pub fn get_free_space(&self) -> u64 {
        self.size_limit - self.size
    }
}

impl Write for TextWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // if there's no file open, create one
        if self.text.is_none() {
            self.create_next_file()?;
        }

        // if there's no space left on the current file, create another one
        // ignore if the file is already empty (if we're already on a new file)
        if (self.size + buf.len() as u64 > self.size_limit) && self.size > 0 {
            self.create_next_file()?;
        }

        if let Some(text) = &mut self.text {
            let bytes_written = text.write(buf)?;
            text.write_all(b"\n\n")?;
            self.size += match u64::try_from(bytes_written) {
                Ok(b) => b,
                Err(e) => {
                    error!(
                        "potential size overflow on lang {} file {} ({:?}): size set to {}",
                        self.lang, self.nb_files, e, self.size_limit
                    );
                    self.size_limit
                }
            };

            Ok(bytes_written)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Could not write to file {} for lang {}",
                    self.nb_files, self.lang
                ),
            ))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match &mut self.text {
            Some(text) => text.flush(),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;
    #[test]
    fn one_file() {
        std::fs::create_dir("tmp_one_file/").unwrap();
        let file_size = 10;
        let mut tw = TextWriter::new(&PathBuf::from("tmp_one_file/"), "en", file_size);
        let text = String::from("helloworld");

        assert_eq!(text.len() as u64, file_size);
        tw.write_all(text.as_bytes()).unwrap();

        let mut file = std::fs::File::open("tmp_one_file/en.txt").unwrap();
        let mut result = String::new();

        file.read_to_string(&mut result).unwrap();

        let mut text = text;

        // to account for newlines
        text.push_str("\n\n");
        assert_eq!(text, result);
        // +2 to account for newlines
        assert_eq!(file_size + 2, file.metadata().unwrap().len());

        std::fs::remove_dir_all("tmp_one_file/").unwrap();
    }

    #[test]
    fn multiple_files() {
        std::fs::create_dir("tmp_multiple/").unwrap();
        let file_size = 10;
        let mut tw = TextWriter::new(&PathBuf::from("tmp_multiple/"), "en", file_size);
        let text = String::from("helloworld");

        for _ in 0..10 {
            tw.write_all(&text.as_bytes()).unwrap();
        }

        let mut text = text;
        text.push_str("\n\n");
        let mut b = String::new();
        for i in 1..=10 {
            b.clear();
            let filename = format!("tmp_multiple/en_part_{}.txt", i);
            let mut file = std::fs::File::open(&filename).unwrap();
            file.read_to_string(&mut b).unwrap();
            assert_eq!(b, text);
            assert_eq!(file_size + 2, file.metadata().unwrap().len());
        }
        std::fs::remove_dir_all("tmp_multiple/").unwrap();
    }

    #[test]
    fn multiple_files_different_sizes() {
        std::fs::create_dir("tmp_multiple_sizes/").unwrap();
        let file_size = 10;
        let mut tw = TextWriter::new(&PathBuf::from("tmp_multiple_sizes/"), "en", file_size);
        let texts = vec![
            "hello\nworld\n", // fits in file 1 (12bytes, overflow but unique document)
            "tiny\ntiny\n",   // fits in file 2 (10bytes, unique (maxed) document)
            "aa\nbb\ncc\n",   // fits in file 3 (9bytes, unique document with 1 byte of free space)
            "short\nshort\n", // fits in file 4 (12bytes, should be in a unique document and not fill up the previous one)
            "medium\n",       // fits in file 5 (7bytes, new document aswell)
            "doc\n",          // fits in file 6 (4bytes, overflowing file 5, 6 bytes of free space)
            "6\n",            // fits in file 6 (2bytes, 4bytes of free space)
            "document7\n",    // fits in file 7 (10bytes, new document, full)
            "0\n",            // fits in file 8 (2bytes, 8bytes of free space)
            "1\n",            // same
            "2\n",            // same
            "3\n",            // same
            "4\n",            // fits in file 8 (2bytes, full)
        ]
        .into_iter()
        .map(String::from);

        // expected data from each part
        let expected_text = vec![
            "hello\nworld\n\n\n",
            "tiny\ntiny\n\n\n",
            "aa\nbb\ncc\n\n\n",
            "short\nshort\n\n\n",
            "medium\n\n\n",
            "doc\n\n\n6\n\n\n",
            "document7\n\n\n",
            "0\n\n\n1\n\n\n2\n\n\n3\n\n\n4\n\n\n",
        ];

        // metadata resets should be at these text indices
        // metadata resets = iterations where we open a new fresh file.
        let mut metadata_resets = vec![0, 1, 2, 3, 4, 5, 7, 8];
        for (idx, text) in texts.enumerate() {
            tw.write_all(&text.as_bytes()).unwrap();

            // if the first write flag is up
            if tw.get_reset_first_write() {
                // remove idx from metadata resets
                let reset_position = metadata_resets
                    .iter()
                    .position(|x| x == &idx)
                    .expect("unexpected metadata reset");
                metadata_resets.remove(reset_position);
                tw.first_write_on_document = false;
            }
        }

        // if code is valid, all metadata resets should have been found.
        assert!(metadata_resets.is_empty());

        let mut b = String::new();
        for i in 1..=8 {
            b.clear();
            let filename = format!("tmp_multiple_sizes/en_part_{}.txt", i);
            let mut f = std::fs::File::open(&filename).unwrap();
            f.read_to_string(&mut b).unwrap();
            let f_size = f.metadata().unwrap().len() as u64;
            assert_eq!(expected_text[i - 1], b);
            assert_eq!(f_size, b.len() as u64);

            std::fs::remove_file(filename).unwrap();
        }
        std::fs::remove_dir_all("tmp_multiple_sizes/").unwrap();
    }
}
