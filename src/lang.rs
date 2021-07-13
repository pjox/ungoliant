//! Language files management.
//!
//! This module contains structs that hold handles to language files
//! and language metadata.
//!
use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

use log::{debug, warn};
use structopt::lazy_static::lazy_static;

lazy_static! {

    /// Holds langs that are available through the OSCAR corpus
    /// Derived from the lang labels from fasttext.
    pub static ref LANG: HashSet<&'static str> = {
        let mut m = HashSet::new();
        m.insert("fr");
        m.insert("af");
        m.insert("als");
        m.insert("am");
        m.insert("an");
        m.insert("ar");
        m.insert("arz");
        m.insert("as");
        m.insert("ast");
        m.insert("av");
        m.insert("az");
        m.insert("azb");
        m.insert("ba");
        m.insert("bar");
        m.insert("bcl");
        m.insert("be");
        m.insert("bg");
        m.insert("bh");
        m.insert("bn");
        m.insert("bo");
        m.insert("bpy");
        m.insert("br");
        m.insert("bs");
        m.insert("bxr");
        m.insert("ca");
        m.insert("cbk");
        m.insert("ce");
        m.insert("ceb");
        m.insert("ckb");
        m.insert("co");
        m.insert("cs");
        m.insert("cv");
        m.insert("cy");
        m.insert("da");
        m.insert("de");
        m.insert("diq");
        m.insert("dsb");
        m.insert("dty");
        m.insert("dv");
        m.insert("el");
        m.insert("eml");
        m.insert("en");
        m.insert("eo");
        m.insert("es");
        m.insert("et");
        m.insert("eu");
        m.insert("fa");
        m.insert("fi");
        m.insert("fr");
        m.insert("frr");
        m.insert("fy");
        m.insert("ga");
        m.insert("gd");
        m.insert("gl");
        m.insert("gn");
        m.insert("gom");
        m.insert("gu");
        m.insert("gv");
        m.insert("he");
        m.insert("hi");
        m.insert("hif");
        m.insert("hr");
        m.insert("hsb");
        m.insert("ht");
        m.insert("hu");
        m.insert("hy");
        m.insert("ia");
        m.insert("id");
        m.insert("ie");
        m.insert("ilo");
        m.insert("io");
        m.insert("is");
        m.insert("it");
        m.insert("ja");
        m.insert("jbo");
        m.insert("jv");
        m.insert("ka");
        m.insert("kk");
        m.insert("km");
        m.insert("kn");
        m.insert("ko");
        m.insert("krc");
        m.insert("ku");
        m.insert("kv");
        m.insert("kw");
        m.insert("ky");
        m.insert("la");
        m.insert("lb");
        m.insert("lez");
        m.insert("li");
        m.insert("lmo");
        m.insert("lo");
        m.insert("lrc");
        m.insert("lt");
        m.insert("lv");
        m.insert("mai");
        m.insert("mg");
        m.insert("mhr");
        m.insert("min");
        m.insert("mk");
        m.insert("ml");
        m.insert("mn");
        m.insert("mr");
        m.insert("mrj");
        m.insert("ms");
        m.insert("mt");
        m.insert("mwl");
        m.insert("my");
        m.insert("myv");
        m.insert("mzn");
        m.insert("nah");
        m.insert("nap");
        m.insert("nds");
        m.insert("ne");
        m.insert("new");
        m.insert("nl");
        m.insert("nn");
        m.insert("no");
        m.insert("oc");
        m.insert("or");
        m.insert("os");
        m.insert("pa");
        m.insert("pam");
        m.insert("pfl");
        m.insert("pl");
        m.insert("pms");
        m.insert("pnb");
        m.insert("ps");
        m.insert("pt");
        m.insert("qu");
        m.insert("rm");
        m.insert("ro");
        m.insert("ru");
        m.insert("rue");
        m.insert("sa");
        m.insert("sah");
        m.insert("sc");
        m.insert("scn");
        m.insert("sco");
        m.insert("sd");
        m.insert("sh");
        m.insert("si");
        m.insert("sk");
        m.insert("sl");
        m.insert("so");
        m.insert("sq");
        m.insert("sr");
        m.insert("su");
        m.insert("sv");
        m.insert("sw");
        m.insert("ta");
        m.insert("te");
        m.insert("tg");
        m.insert("th");
        m.insert("tk");
        m.insert("tl");
        m.insert("tr");
        m.insert("tt");
        m.insert("tyv");
        m.insert("ug");
        m.insert("uk");
        m.insert("ur");
        m.insert("uz");
        m.insert("vec");
        m.insert("vep");
        m.insert("vi");
        m.insert("vls");
        m.insert("vo");
        m.insert("wa");
        m.insert("war");
        m.insert("wuu");
        m.insert("xal");
        m.insert("xmf");
        m.insert("yi");
        m.insert("yo");
        m.insert("yue");
        m.insert("zh");

        m
    };
}

/// Holds language files handlers
///
/// For each available language, a file is created
/// and is writeable via the handlers.
///
/// When using [LangFiles], be aware that ~160 files will stay open while the structure is not dropped.
///
// TODO: replace this with an alias to HashMap?
// This way we don't need to manually bind HashMap methods
// TODO: both constructors have the same code, use a "factory"?
pub struct LangFiles {
    handles: HashMap<&'static str, File>,
}

impl LangFiles {
    /// open a file handle for each language
    #[deprecated(
        since = "0.1.0",
        note = "Please use the crate::writing::LangFiles structure instead"
    )]
    pub fn new(src: &Path) -> Result<Self, std::io::Error> {
        warn!("Deprecated in favor of crate::writing::LangFiles!");
        let mut options = OpenOptions::new();
        options.read(true).append(true).create(true);
        let mut handles = HashMap::new();
        for lang in LANG.iter() {
            let mut file_path: PathBuf = [src, &Path::new(*lang)].iter().collect();
            file_path.set_extension("txt");
            debug!("creating/opening {:?}", file_path);
            let fh = options.clone().open(file_path)?;
            handles.insert(*lang, fh);
        }

        Ok(LangFiles { handles })
    }

    /// binds to [HashMap::get].
    pub fn get(&self, key: &'static str) -> Option<&File> {
        self.handles.get(key)
    }
}
