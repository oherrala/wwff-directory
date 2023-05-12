//! # A parser for WWFF directory CSV file
//!
//! The CSV file can be found from <https://wwff.co/wwff-data/wwff_directory.csv>

use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::Path;

use serde::{Deserialize, Deserializer};
use tinystr::TinyAsciiStr;
use tracing::instrument;

/// The WWFF official unique identifying reference number
pub type Reference = TinyAsciiStr<12>;

/// The directory containing WWFF information
pub type WwffDirectory = BTreeMap<Reference, Entry>;

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
    #[serde(rename(deserialize = "IUCNcat"), deserialize_with = "deserialize_tinystr")]
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
    #[serde(rename(deserialize = "changeLog"), deserialize_with = "deserialize_string_opt")]
    pub changelog: Option<String>,
    #[serde(rename(deserialize = "reviewFlag"))]
    pub review_flag: u8,
    #[serde(rename(deserialize = "specialFlags"), deserialize_with = "deserialize_string_opt")]
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
    pub last_act: Option<chrono::NaiveDate>,
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
#[instrument(skip(path))]
pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<WwffDirectory> {
    read(csv::Reader::from_path(path)?)
}

/// Read CSV file from given reader
#[instrument(skip(reader))]
pub fn from_reader<R: Read>(reader: R) -> io::Result<WwffDirectory> {
    read(csv::Reader::from_reader(reader))
}

#[instrument(skip(rdr))]
fn read<R: Read>(mut rdr: csv::Reader<R>) -> io::Result<WwffDirectory> {
    let mut result = BTreeMap::new();
    let ts = std::time::Instant::now();
    for entry in rdr.deserialize() {
        match entry {
            Ok(e) => {
                let e: Entry = e;
                result.insert(e.reference.to_owned(), e);
            }
            Err(err) => {
                tracing::error!("Skipping invalid row. Error: {err}");
                continue;
            }
        }
    }
    tracing::debug!(
        "Reading WWFF directory ({} entries) took {} ms.",
        result.len(),
        ts.elapsed().as_millis()
    );
    Ok(result)
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
