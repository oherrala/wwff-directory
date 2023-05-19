//! # A parser for WWFF directory CSV file
//!
//! There are three ways to use this crate:
//!
//!  1. Read WWFF directory directly from file with [from_path] function.
//!  2. Read WWFF directory from given reader with [from_reader] function.
//!  3. And lastly the most complex option is to use feature "downloader" which enables functions [WwffDirectory::from_download] and [WwffDirectory::try_download_update].
//!
//! The official CSV file can be found from <https://wwff.co/wwff-data/wwff_directory.csv>.

use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::Path;

use serde::{Deserialize, Deserializer};
use tinystr::TinyAsciiStr;
use tracing::instrument;

#[cfg(feature = "downloader")]
mod downloader;

/// WWFF Unique Identifying Reference number
///
/// From [WWFF Global
/// Rules](https://wwff.co/rules-faq/how-to-activate-a-wwff-reference/) version
/// 5.9, section 3.4:
///
/// > Each WWFF designated park and/or protected nature area is issued with a
/// > unique alpha numeric identifying reference number. The reference number
/// > consists of:
/// > - the ITU allocated prefix;
/// > - FF for Flora and Fauna;
/// > - and a unique identifying number comprising four digits
/// >   - e.g. ONFF-0010
pub type Reference = TinyAsciiStr<12>;

type WwffMap = BTreeMap<Reference, Entry>;

/// The directory containing WWFF information
#[derive(Debug)]
pub struct WwffDirectory {
    map: WwffMap,
    #[cfg(feature = "downloader")]
    downloader: downloader::Downloader,
}

impl WwffDirectory {
    /// Read CSV file from given [Path]
    #[instrument(fields(path = %path.as_ref().to_string_lossy()))]
    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<WwffDirectory> {
        let map = read(csv::Reader::from_path(path)?)?;
        Ok(Self {
            map,
            #[cfg(feature = "downloader")]
            downloader: downloader::Downloader::new(),
        })
    }

    /// Read CSV file from given reader
    #[instrument(skip(reader))]
    pub fn from_reader<R: Read>(reader: R) -> io::Result<WwffDirectory> {
        let map = read(csv::Reader::from_reader(reader))?;
        Ok(Self {
            map,
            #[cfg(feature = "downloader")]
            downloader: downloader::Downloader::new(),
        })
    }

    /// Download WWFF directory from it's original source.
    ///
    /// After this initial download it's possible to update the WWFF directory
    /// in-place with [WwffDirectory::try_download_update] function.
    #[cfg(feature = "downloader")]
    #[instrument]
    pub async fn from_download() -> io::Result<WwffDirectory> {
        let mut downloader = downloader::Downloader::new();
        let map = downloader.download().await?;
        match map {
            Some(map) => Ok(Self { map, downloader }),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "initial download failed",
            )),
        }
    }

    /// Try to download updated version of WWFF directory. If there's new
    /// version available then the directory is updated automatically.
    #[cfg(feature = "downloader")]
    #[instrument(skip(self))]
    pub async fn try_download_update(&mut self) -> io::Result<()> {
        if let Some(map) = self.downloader.download().await? {
            self.map = map;
        }
        Ok(())
    }

    /// Search WWFF directory for reference.
    #[instrument]
    pub fn search_reference(&self, s: &str) -> Option<&Entry> {
        let reference = TinyAsciiStr::from_str(s).ok()?.to_ascii_uppercase();
        self.map.get(&reference)
    }
}

/// A single WWFF entity entry
#[derive(Debug, Deserialize)]
pub struct Entry {
    pub reference: Reference,
    #[serde(deserialize_with = "deserialize_status")]
    pub status: Status,
    pub name: String,
    pub program: TinyAsciiStr<12>,
    pub dxcc: TinyAsciiStr<8>,
    pub state: TinyAsciiStr<8>,
    pub county: TinyAsciiStr<8>,
    pub continent: TinyAsciiStr<2>,
    #[serde(deserialize_with = "deserialize_tinystr")]
    pub iota: Option<TinyAsciiStr<8>>,
    #[serde(
        rename(deserialize = "iaruLocator"),
        deserialize_with = "deserialize_tinystr"
    )]
    pub iaru_locator: Option<TinyAsciiStr<12>>,
    #[serde(deserialize_with = "deserialize_f32_opt")]
    pub latitude: Option<f32>,
    #[serde(deserialize_with = "deserialize_f32_opt")]
    pub longitude: Option<f32>,
    /// International Union for Conservation of Nature (IUCN) category
    #[serde(
        rename(deserialize = "IUCNcat"),
        deserialize_with = "deserialize_tinystr"
    )]
    pub iucn_category: Option<TinyAsciiStr<12>>,
    #[serde(
        rename(deserialize = "validFrom"),
        deserialize_with = "deserialize_date_opt"
    )]
    pub valid_from: Option<chrono::NaiveDate>,
    #[serde(
        rename(deserialize = "validTo"),
        deserialize_with = "deserialize_date_opt"
    )]
    pub valid_to: Option<chrono::NaiveDate>,
    pub notes: String,
    #[serde(rename(deserialize = "lastMod"))]
    pub last_modified: String,
    #[serde(
        rename(deserialize = "changeLog"),
        deserialize_with = "deserialize_string_opt"
    )]
    pub changelog: Option<String>,
    #[serde(rename(deserialize = "reviewFlag"))]
    pub review_flag: u8,
    #[serde(
        rename(deserialize = "specialFlags"),
        deserialize_with = "deserialize_string_opt"
    )]
    pub special_flags: Option<String>,
    #[serde(deserialize_with = "deserialize_string_opt")]
    pub website: Option<String>,
    #[serde(deserialize_with = "deserialize_string_opt")]
    pub country: Option<String>,
    #[serde(deserialize_with = "deserialize_string_opt")]
    pub region: Option<String>,
    #[serde(rename(deserialize = "dxccEnum"))]
    pub dxcc_enum: Option<u16>,
    #[serde(rename(deserialize = "qsoCount"))]
    pub qso_count: Option<u32>,
    #[serde(
        rename(deserialize = "lastAct"),
        deserialize_with = "deserialize_date_opt"
    )]
    pub last_activity: Option<chrono::NaiveDate>,
}

/// Status of the [Entry]
#[derive(Debug)]
pub enum Status {
    Active,
    Deleted,
    National,
    Proposed,
}

/// Read CSV file from given [Path]
pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<WwffDirectory> {
    WwffDirectory::from_path(path)
}

/// Read CSV file from given reader
pub fn from_reader<R: Read>(reader: R) -> io::Result<WwffDirectory> {
    WwffDirectory::from_reader(reader)
}

#[instrument(skip(rdr))]
fn read<R: Read>(mut rdr: csv::Reader<R>) -> io::Result<WwffMap> {
    let mut map = BTreeMap::new();
    let ts = std::time::Instant::now();

    for entry in rdr.deserialize() {
        match entry {
            Ok(e) => {
                let e: Entry = e;
                let reference = e.reference.to_ascii_uppercase();
                map.insert(reference, e);
            }
            Err(err) => {
                tracing::error!("Skipping invalid row. Error: {err}");
                continue;
            }
        }
    }

    tracing::debug!(
        "Reading WWFF directory ({} entries) took {} ms.",
        map.len(),
        ts.elapsed().as_millis()
    );

    Ok(map)
}

fn deserialize_f32_opt<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(f32::deserialize(deserializer).ok())
}

fn deserialize_date_opt<'de, D>(deserializer: D) -> Result<Option<chrono::NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(chrono::NaiveDate::deserialize(deserializer).ok())
}

fn deserialize_status<'de, D>(deserializer: D) -> Result<Status, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(match s.to_ascii_lowercase().as_str() {
        "active" => Status::Active,
        "deleted" => Status::Deleted,
        "national" => Status::National,
        "proposed" => Status::Proposed,
        _ => {
            return Err(serde::de::Error::custom(format!(
                "Unknown WWFF status \"{s}\""
            )))
        }
    })
}

fn deserialize_string_opt<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if s.is_empty() || s == "-" || s == "n/a" {
        return Ok(None);
    }

    Ok(Some(s))
}

fn deserialize_tinystr<'de, D, const N: usize>(
    deserializer: D,
) -> Result<Option<TinyAsciiStr<N>>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if s.is_empty() || s == "-" || s == "n/a" {
        return Ok(None);
    }

    // Skip known issue
    if s == "Región 1" {
        return Ok(None);
    }

    if let Ok(s) = TinyAsciiStr::from_str(s.trim()) {
        return Ok(Some(s));
    }

    Err(serde::de::Error::custom(format!(
        "Couldn't deserialize \"{s}\" to TinyAsciiStr"
    )))
}
