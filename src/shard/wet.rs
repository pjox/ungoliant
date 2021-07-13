use std::{fs::File, io::BufReader, path::Path};

use crate::error::Error;
use flate2::read::MultiGzDecoder;
use std::io::BufRead;
use warc::WarcReader;

/// Wet/Shard instance, generic over reader type.
///
/// This genericity enables Ungoliant to potentially
/// manage compressed and decompressed `wet` files.
///
/// Be aware that CommonCrawl files are gzipped and need
/// a multi gz decoder (such as [MultiGzDecoder]).
pub struct Wet<T> {
    reader: WarcReader<T>,
}

/// Wet reader using [MultiGzDecoder] over a [File].
impl Wet<BufReader<MultiGzDecoder<File>>> {
    /// Create a new reader from a gzipped WET file.
    pub fn from_path_gzip<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let gzip_file = File::open(path)?;
        let gzip_stream = MultiGzDecoder::new(gzip_file);

        // we use a different reader from the default one in the warc crate to
        // manage multipart gzipped content.
        let bufreader = BufReader::new(gzip_stream);

        let reader = WarcReader::new(bufreader);

        Ok(Self { reader })
    }
}

#[allow(dead_code)]
impl<T: BufRead> Wet<T> {
    pub fn new(reader: T) -> Self {
        Self {
            reader: WarcReader::new(reader),
        }
    }
}

impl<R: BufRead> Iterator for Wet<R> {
    type Item = Result<warc::RawRecord, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.reader.next() {
            match n {
                Ok(record) => Some(Ok(record)),
                Err(e) => Some(Err(Error::Warc(e))),
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use serde_json;
    use std::collections::HashMap;
    use warc::header::WarcHeader;

    use super::Wet;

    #[test]
    #[ignore]
    fn test_init() {
        let _ = Wet::from_path_gzip("results/0.txt.gz").unwrap();
    }

    #[test]
    #[ignore]
    fn test_metadata() {
        // see https://github.com/commoncrawl/nutch/blob/cc/src/java/org/commoncrawl/util/WarcWriter.java#L53
        // for explicit list of fields?
        // there is warc/1.0 fields + some other CC specific ones
        // although records are marked warc/1.0, they follow 1.1 spec
        // See https://iipc.github.io/warc-specifications/specifications/warc-format/warc-1.1/#named-fields
        // for a better explanation of fields
        let shard = Wet::from_path_gzip("results/0.txt.gz").unwrap();

        for (idx, record) in shard.enumerate().skip(1).take(4) {
            let record = record.unwrap();
            let headers: HashMap<WarcHeader, String> = record
                .headers
                .into_iter()
                .map(|(k, v)| (k, String::from_utf8_lossy(&v).to_string()))
                .collect();
            println!("record {}", idx);
            println!(
                "headers: {}",
                serde_json::to_string_pretty(&headers).unwrap()
            );
        }
    }

    #[test]
    fn deserialize_real_metadata() {
        let headers_json = r#"{
            "warc-type": "conversion",
            "warc-target-uri": "http://011758.www--haoxpj.vip/",
            "warc-identified-content-language": "zho",
            "content-type": "text/plain",
            "warc-block-digest": "sha1:UEU5IYZ7O36BG22UJNN5UXYBT445XRD7",
            "warc-date": "2021-02-24T17:02:28Z",
            "content-length": "6231",
            "warc-refers-to": "<urn:uuid:92f90e43-3e99-4d44-a194-1492137d7bf4>",
            "warc-record-id": "<urn:uuid:c7f19cbd-e348-48ff-9a92-4852b114b6db>"
          }"#;

        let headers: HashMap<WarcHeader, String> = serde_json::from_str(&headers_json).unwrap();
        println!("{:?}", headers);
    }

    #[test]
    fn serde_metadata() {
        let mut headers: HashMap<WarcHeader, String> = HashMap::new();
        headers.insert(WarcHeader::WarcType, "conversion".to_string());
        headers.insert(WarcHeader::ContentLength, "6231".to_string());
        headers.insert(
            WarcHeader::Unknown("warc-identified-content-language".to_string()),
            "zho".to_string(),
        );

        let headers_json = serde_json::to_string(&headers).unwrap();
        let headers_deserialized = serde_json::from_str(&headers_json).unwrap();

        assert_eq!(headers, headers_deserialized);
    }
}
