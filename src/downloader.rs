use std::io;

use reqwest::header::{HeaderValue, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use thiserror::Error;
use tracing::instrument;

use crate::WwffMap;

const WWFF_DIRECTORY_URL: &str = "https://wwff.co/wwff-data/wwff_directory.csv";
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub(crate) struct Downloader {
    client: reqwest::Client,
    last_modified: Option<HeaderValue>,
    etag: Option<HeaderValue>,
}

impl Downloader {
    #[instrument]
    pub fn new() -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .build()
            .unwrap();

        Self {
            client,
            last_modified: None,
            etag: None,
        }
    }

    #[instrument(skip(self))]
    pub async fn download(&mut self) -> Result<Option<WwffMap>, DownloaderError> {
        let client = &self.client;

        let mut request = client.get(WWFF_DIRECTORY_URL);

        if let Some(last_modified) = &self.last_modified {
            tracing::debug!("Adding If-Modified-Since header: {last_modified:?}");
            request = request.header(IF_MODIFIED_SINCE, last_modified);
        }

        if let Some(etag) = &self.etag {
            tracing::debug!("Adding If-None-Match header: {etag:?}");
            request = request.header(IF_NONE_MATCH, etag);
        }

        let resp = request.send().await?;

        // Not modified since last request
        if resp.status() == 304 {
            tracing::debug!("wwff_directory.csv not modified. Bandwidth saved.");
            return Ok(None);
        }

        if resp.status() != 200 {
            tracing::debug!("Some error {}", resp.status());
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("HTTP response returned status code {}", resp.status()),
            )
            .into());
        }

        self.last_modified = resp.headers().get(LAST_MODIFIED).cloned();
        self.etag = resp.headers().get(ETAG).cloned();

        let text = resp.text().await?;
        let wwff_map = crate::read(csv::Reader::from_reader(text.as_bytes()))?;

        Ok(Some(wwff_map))
    }
}

#[derive(Error, Debug)]
pub(crate) enum DownloaderError {
    #[error("IO error")]
    Io(#[from] io::Error),
    #[error("HTTP error")]
    Http(#[from] reqwest::Error),
}

impl From<DownloaderError> for io::Error {
    fn from(err: DownloaderError) -> Self {
        match err {
            DownloaderError::Io(err) => err,
            DownloaderError::Http(err) => io::Error::new(io::ErrorKind::Other, err),
        }
    }
}
