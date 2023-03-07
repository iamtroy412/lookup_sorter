use assert_fs::prelude::FileWriteStr;
use log::{info, warn, debug};
use reqwest::header::HeaderMap;
use reqwest::redirect::Policy;
use anyhow::{Context, Result};
use serde::Serialize;
use dns_lookup::lookup_host;
use std::time::Duration;
use std::path::PathBuf;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::net::IpAddr;

#[derive(Debug, Serialize)]
pub struct Site {
    pub host: String,
    pub addrs: Vec<IpAddr>,
    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,
}

// build_sites returns a vector of Site structs, one for each name in the input file.
pub fn build_sites(input_path: &PathBuf) -> Result<Vec<Site>, anyhow::Error> {
    info!("Opening `{}` for reading", &input_path.display());
    let file = File::open(input_path).with_context(||
        format!("Failed to open `{}`", &input_path.display()))?;
    
    let reader = BufReader::new(file);

    let mut sites = Vec::new();

    info!("Reading lines from `{}` into `Site` structs", &input_path.display());
    for line in reader.lines() {
        let line = line.with_context(||
            format!("Failed to read `{}`", &input_path.display()))?;

        debug!("`&line`: {:?}", &line);
        sites.push(Site {
            host: line.trim().to_string(),
            addrs: Vec::new(),
            headers: HeaderMap::new(),
        });
    }

    debug!("`&sites`: {:?}", &sites);
    Ok(sites)
}

#[test]
fn test_build_sites() {
    let file = assert_fs::NamedTempFile::new("sample.txt").unwrap();
    file.write_str("google.com\nasfasdf.asdf\nyahoo.com").unwrap();

    let base_case = vec![
        Site { host: "google.com".to_owned(), addrs: Vec::new(), headers: HeaderMap::new() },
        Site { host: "asfasdf.asdf".to_owned(), addrs: Vec::new(), headers: HeaderMap::new() },
        Site { host: "yahoo.com".to_owned(), addrs: Vec::new(), headers: HeaderMap::new() }
    ];

    let result = build_sites(&file.path().to_path_buf()).unwrap();
    assert_eq!(base_case[0].host, result[0].host);
    assert_eq!(base_case[1].host, result[1].host);
    assert_eq!(base_case[2].host, result[2].host);
}

// look_and_connect takes a Site struct, resolves the IP address of the name, and then
// tries to connect to it and record the headers.
pub fn look_and_connect(site: &Site) -> Result<(Vec<IpAddr>, HeaderMap), anyhow::Error> {
    let mut addresses = Vec::<IpAddr>::new();
    let mut headers = HeaderMap::new();
    
    info!("Running DNS lookup on `{}`...", &site.host);
    match lookup_host(&site.host) {
        Ok(addrs) => {
            debug!("`&addrs`: {:?}", &addrs);
            addresses = addrs;
            let client = &reqwest::blocking::Client::builder().redirect(Policy::none()).build()?;
            info!("Connecting to `{}`...", &site.host);
            match client.get(format!("http://{}", &site.host)).timeout(Duration::from_secs(3)).send(){
                Ok(resp) => {
                    debug!("`&response.headers`: {:?}", &resp.headers());
                    headers = resp.headers().clone();
                },
                Err(err) => {
                    warn!("Unable to make connection: {:?}", &err);
                }
            };
        },
        Err(e) => warn!("`&site.host`: {} Error: {}", &site.host, e),
    };

    Ok((addresses, headers))

}

#[test]
fn test_look_and_connect() {
    let mut site = Site { host: "google.com".to_owned(), addrs: Vec::new(), headers: HeaderMap::new() };

    (site.addrs, site.headers) = look_and_connect(&site).unwrap();
    
    assert!(site.addrs.len() > 0);
    assert!(site.headers.len() > 0);
}