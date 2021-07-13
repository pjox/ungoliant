//! Document-level structures and operations
//!
//! A Document is analogous to a WARC Record, but a Document is filtered and its lines are identified (contains language data).
//!
//! - A [Document] can be splitted into [Piece]s. A [Piece] holds several sentences that share the same language.
//! - Sentences from a [Piece] are contiguous in a document.
//! - A [Piece] can be transformed into a [MergedPiece], which contains a single [String] that joins all sentences from the [Piece].
//!
//! A [Document] can be transformed into a [Vec<MergedPieces>] in two ways:
//! - by using [Document::into_pieces] which transforms each [Piece] into a [MergedPiece]
//! - by using [Document::into_pieces_lang] which merges same-language [Piece] into a unique one.
//!
//! into_pieces can be useful if order of paragraphs is important and you wish to reconstruct documents, but will yield datasets that are not compatible with OSCAR2018.
//!  
use crate::error::Error;
use crate::pipeline::oscar_metadata::chunks;
use crate::pipeline::Metadata;
// use crate::pipeline::oscar_metadata::metadata::Metadata;
// use log::warn;
use log::warn;
use std::collections::HashMap;
use std::convert::TryFrom;
// use std::convert::TryFrom;
// use std::string::FromUtf8Error;
use warc::header::WarcHeader;

/// represents a whole docuement, that is:
/// - its header, as provided by warc library
/// - its sentences, as an array of Strings
/// - its identifications (one by line)
///
/// a document is a filtered, annotated version of a record
#[derive(Debug)]
pub struct Document {
    headers: HashMap<WarcHeader, Vec<u8>>,
    sentences: Vec<String>,
    identifications: Vec<&'static str>,
}

/// A piece is a series of sentences from a same document
/// that share the same language.
#[derive(Debug)]
struct Piece {
    headers: HashMap<WarcHeader, Vec<u8>>,
    sentences: Vec<String>,
    identification: &'static str,
}

/// Holds a merged-down version of Piece, where sentences are merged into a single String
#[derive(Debug, Clone)]
pub struct MergedPiece {
    pub headers: HashMap<WarcHeader, Vec<u8>>,
    pub sentences: String,
    pub nb_sentences: usize,
    pub identification: &'static str,
}

impl MergedPiece {
    /// create a new merged piece
    /// nb_sentences is computed from sentences
    pub fn new(
        headers: HashMap<WarcHeader, Vec<u8>>,
        sentences: Vec<String>,
        identification: &'static str,
    ) -> Self {
        let nb_sentences = sentences.len();
        let sentences = sentences.join("\n");
        MergedPiece {
            headers,
            sentences,
            nb_sentences,
            identification,
        }
    }

    pub fn identification(&self) -> &'static str {
        &self.identification
    }
}

impl From<Piece> for MergedPiece {
    /// create a new merged piece from a piece
    ///
    /// discards language information
    fn from(piece: Piece) -> Self {
        MergedPiece::new(piece.headers, piece.sentences, piece.identification)
    }
}

/// Fraction of a larger OSCAR Part
///
/// contains the concatenation of MergedPieces of a same language
/// properly offseted and space separated:
/// - first offset at 0
/// - first item is of nb_0 length
/// - one newline
/// - next offset at nb_0+1
#[derive(Debug)]
pub struct PartChunk {
    pub metadata: Vec<Metadata>,
    pub body: String,
}

impl PartChunk {
    /// Create a new PartChunk.
    /// Note that the same language constraint is not checked.
    /// It must be done before creating a PartChunk.
    pub fn new(merged_pieces: Vec<MergedPiece>) -> Result<Self, Error> {
        let mut metadata = Vec::new();
        let mut body = String::new();

        let mut cur_offset = 0;
        let merged_pieces_len = merged_pieces.len();
        for (idx, piece) in merged_pieces.into_iter().enumerate() {
            //build metadata
            let mut m = Metadata::try_from(piece.headers)?;
            m.offset = cur_offset;
            m.nb_sentences = piece.nb_sentences;

            body += &piece.sentences;

            // only add newline between paragraphs,
            // don't add one at the end of the partchunk.
            if idx < merged_pieces_len - 1 {
                body += "\n\n";

                // bump 1 to account for newline
                cur_offset += m.nb_sentences + 1;
            }

            metadata.push(m);
        }

        Ok(Self { metadata, body })
    }

    /// updates offsets.
    ///
    /// This offsets the metadata's `offset` fields by the provided `offset` value.
    /// Returns the offset to use for future writes
    pub fn bump_offsets(&mut self, offset: usize) -> Option<usize> {
        self.metadata.iter_mut().for_each(|m| m.offset += offset);
        match self.metadata.last() {
            Some(m) => Some(m.offset + m.nb_sentences + 1),
            None => {
                warn!("no metadata!");
                None
            }
        }
    }
}

#[allow(dead_code)]
impl Document {
    /// create a new document
    ///
    /// returns an error if sentences and identifications
    /// are of different length
    pub fn new(
        headers: HashMap<WarcHeader, Vec<u8>>,
        sentences: Vec<String>,
        identifications: Vec<&'static str>,
    ) -> Result<Self, Error> {
        if sentences.len() != identifications.len() {
            return Err(Error::Custom(
                "different number of sentences and identifications".to_string(),
            ));
        }

        Ok(Self {
            headers,
            sentences,
            identifications,
        })
    }

    /// chops the document into a vector of [MergedPiece]
    pub fn into_merged_pieces(self) -> Vec<MergedPiece> {
        let pieces = self.into_pieces();
        pieces.into_iter().map(MergedPiece::from).collect()
    }

    /// chops the document into a vector of [MergedPiece]
    /// while merging same-language sentences into a single merged piece.
    pub fn into_merged_pieces_lang(self) -> Vec<MergedPiece> {
        let pieces = self.into_pieces_lang();
        pieces.into_iter().map(MergedPiece::from).collect()
    }

    /// chops the document into a vector of [Piece].
    fn into_pieces(self) -> Vec<Piece> {
        let language_chunks = chunks::group_by(self.identifications.clone());
        let mut pieces = Vec::new();
        for (language, chunks_indices) in language_chunks {
            let new_pieces = chunks_indices.into_iter().map(|chunk_index| Piece {
                headers: self.headers.clone(),
                sentences: self.sentences[chunk_index].to_vec(),
                identification: language,
            });
            pieces.extend(new_pieces);
        }

        pieces
    }

    /// chops the document into a vector of [Piece],
    /// while grouping same-language sentences into a single piece.
    fn into_pieces_lang(self) -> Vec<Piece> {
        let language_chunks = chunks::group_by(self.identifications.clone());
        let mut hm: HashMap<&'static str, Vec<String>> = HashMap::new();
        for (language, chunks_indices) in language_chunks {
            let e = hm.entry(language).or_insert_with(Vec::new);
            for chunk_index in chunks_indices {
                e.append(&mut self.sentences[chunk_index].to_vec());
            }
        }

        hm.into_iter()
            .map(|(lang, sentences)| Piece {
                headers: self.headers.clone(),
                sentences,
                identification: lang,
            })
            .collect()
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    fn gen_test() -> (HashMap<WarcHeader, Vec<u8>>, Vec<String>, Vec<&'static str>) {
        let headers = vec![(WarcHeader::ContentLength, vec![0])]
            .into_iter()
            .collect();
        let sentences = vec![
            "Bonjour je suis une phrase française 0",
            "Bonjour je suis une phrase française 1",
            "Bonjour je suis une phrase française 2",
            "hi i'm an english sentence 3",
            "Bonjour je suis une phrase française 4",
            "hi i'm an english sentence 5",
            "ich bin eine Berliner 6",
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect();

        let identifications = vec!["fr", "fr", "fr", "en", "fr", "en", "de"];

        (headers, sentences, identifications)
    }

    fn gen_records() -> Vec<Document> {
        let sentences_1: Vec<String> = vec![
            "1 Document intégralement en français",
            "2 Document intégralement en français",
            "3 Document intégralement en français",
            "4 Document intégralement en français",
            "5 Document intégralement en français",
            "6 Document intégralement en français",
            "7 Document intégralement en français",
            "8 Document intégralement en français",
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect();

        let identifications_1 = vec!["fr"; sentences_1.len()];

        let sentences_2: Vec<String> = vec![
            "1 English-only document",
            "2 English-only document",
            "3 English-only document",
            "4 English-only document",
            "5 English-only document",
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect();
        let identifications_2 = vec!["en"; sentences_2.len()];

        let sentences_3 = vec![
            "1  Document partiellement en français",
            "2  Partially english document",
            "3  Document partiellement en français",
            "4  Document partiellement en français",
            "5  Document partiellement en français",
            "6  Partially english document",
            "7  Partially english document",
            "8  Partially english document",
            "9  Document partiellement en français",
            "10 Document partiellement en français",
            "11 Partially english document",
            "12 Partially english document",
            "13 Partially english document",
            "14 Partially english document",
            "15 Document partiellement en français",
            "16 Document partiellement en français",
            "17 Document partiellement en français",
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect();

        let identifications_3 = vec![
            "fr", "en", "fr", "fr", "fr", "en", "en", "en", "fr", "fr", "en", "en", "en", "en",
            "fr", "fr", "fr",
        ];

        let sentences_4 = vec![
            "1  Alternatively english",
            "2  Alternativement français",
            "3  Alternatively english",
            "4  Alternativement français",
            "5  Alternatively english",
            "6  Alternativement français",
            "7  Alternatively english",
            "8  Alternativement français",
            "9  Alternatively english",
            "10 Alternativement français",
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect();

        let identifications_4 = vec!["fr", "en", "fr", "en", "fr", "en", "fr", "en", "fr", "en"];

        let meta_1 = vec![(WarcHeader::RecordID, vec![1])].into_iter().collect();
        let meta_2 = vec![(WarcHeader::RecordID, vec![2])].into_iter().collect();
        let meta_3 = vec![(WarcHeader::RecordID, vec![3])].into_iter().collect();
        let meta_4 = vec![(WarcHeader::RecordID, vec![4])].into_iter().collect();

        let mut ret = Vec::new();
        ret.push(Document::new(meta_1, sentences_1, identifications_1).unwrap());
        ret.push(Document::new(meta_2, sentences_2, identifications_2).unwrap());
        ret.push(Document::new(meta_3, sentences_3, identifications_3).unwrap());
        ret.push(Document::new(meta_4, sentences_4, identifications_4).unwrap());

        ret
    }

    #[test]
    fn document_new() {
        let (headers, sentences, identifications) = gen_test();
        let d = Document::new(headers, sentences, identifications);
        assert!(d.is_ok());
    }

    #[test]
    fn document_new_incorrect_length() {
        let (headers, sentences, mut identifications) = gen_test();
        identifications.pop();
        let d = Document::new(headers, sentences, identifications);
        assert!(d.is_err());
    }

    #[test]
    fn document_into_pieces() {
        let (headers, sentences, ids) = gen_test();
        let d = Document::new(headers.clone(), sentences, ids).unwrap();
        // let pieces = d.into_lang_pieces();
        // // println!("{:#?}", pieces);

        // assert!(pieces.pieces().iter().all(|x| x.headers() == &headers));
        // for (result, expected) in pieces.iter().zip(ids.iter()) {
        //     assert_eq!(&result.identification(), expected);
        // }
    }

    #[test]
    fn document_by_lang() {
        let (headers, sentences, identifications) = gen_test();
        let d = Document::new(headers.clone(), sentences.clone(), identifications).unwrap();
        let merged_pieces = d.into_merged_pieces();
        println!("{:?}", sentences);
        println!("{:#?}", merged_pieces);
    }

    #[test]
    fn merge_to_parts() {
        let docs = gen_records();
        let docs_merged = docs
            .into_iter()
            .map(|doc| doc.into_merged_pieces())
            .collect::<Vec<Vec<MergedPiece>>>();
        println!("{:#?}", docs_merged);
    }

    // #[test]
    // fn merge_to_partschunks() {
    //     let mut hm: HashMap<&'static str, Vec<MergedPiece>> = HashMap::new();
    //     let docs = gen_records();
    //     let docs_merged = docs
    //         .into_iter()
    //         .map(|doc| doc.into_merged_pieces_lang())
    //         .flatten()
    //         .collect::<Vec<MergedPiece>>();

    //     for piece in docs_merged {
    //         let e = hm.entry(piece.identification).or_insert(Vec::new());
    //         e.push(piece);
    //     }
    //     // println!("{:#?}", hm);

    //     for (lang, pieces) in hm {
    //         let pc = PartChunk::new(pieces).unwrap();
    //         println!("{:#?}", pc.metadata);
    //         println!(
    //             "{:#?}",
    //             pc.body.lines().enumerate().collect::<Vec<(usize, &str)>>()
    //         );
    //     }
    // }
}
