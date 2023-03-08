use assert_fs::prelude::FileWriteStr;
use log::{info, warn, debug};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::redirect::Policy;
use anyhow::{Context, Result};
use serde::Serialize;
use dns_lookup::lookup_host;
use std::time::Duration;
use std::path::PathBuf;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::net::{IpAddr, Ipv4Addr};
use ipnet::Ipv4Net;

#[derive(Debug, Serialize)]
pub struct Site {
    pub host: String,
    pub addrs: Vec<IpAddr>,
    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,
    pub bigip: Option<String>,
}

pub fn build_subnets(input_path: &PathBuf) -> Result<Vec<Ipv4Net>, anyhow::Error> {
    info!("Opening `{}` for reading", &input_path.display());
    let file = File::open(input_path).with_context(||
        format!("Failed to open `{}`", &input_path.display()))?;
    
    let reader = BufReader::new(file);

    let mut subnets: Vec<Ipv4Net> = Vec::new();

    info!("Reading lines from `{}` into `subnets` vec", &input_path.display());
    for line in reader.lines() {
        let line = line.with_context(||
            format!("Failed to read `{}`", &input_path.display()))?;

        debug!("`&line`: {:?}", &line);
        match &line.parse::<Ipv4Net>() {
            Ok(net) => subnets.push(*net),
            Err(e) => { warn!("{}", e); }
        }
    }
    debug!("`subnets`: {:?}", &subnets);
    Ok(subnets)
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
            bigip: None,
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
        Site { host: "google.com".to_owned(), addrs: Vec::new(), headers: HeaderMap::new(), bigip: None },
        Site { host: "asfasdf.asdf".to_owned(), addrs: Vec::new(), headers: HeaderMap::new(), bigip: None },
        Site { host: "yahoo.com".to_owned(), addrs: Vec::new(), headers: HeaderMap::new(), bigip: None },
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
    let mut site = Site { host: "google.com".to_owned(), addrs: Vec::new(), headers: HeaderMap::new(), bigip: None };

    (site.addrs, site.headers) = look_and_connect(&site).unwrap();
    
    assert!(site.addrs.len() > 0);
    assert!(site.headers.len() > 0);
}

pub fn look_and_connect2(site: &Site) -> Result<(Vec<IpAddr>, HeaderMap), anyhow::Error> {
    info!("Running DNS lookup on `{}`...", &site.host);
    let ips = match lookup_host(&site.host) {
        Ok(addrs) =>
            addrs,
        Err(e) => {
            warn!("Unable to make connection: {:?}", &e);
            Vec::<IpAddr>::new()
        }
    };

    let client = &reqwest::blocking::Client::builder().redirect(Policy::none()).build()?;
    info!("Connecting to `{}`...", &site.host);
    let hddrs = match client.get(format!("http://{}", &site.host)).timeout(Duration::from_secs(3)).send(){
        Ok(resp) => {
            debug!("`&response.headers`: {:?}", &resp.headers());
            resp.headers().clone()
        },
        Err(err) => {
            warn!("Unable to make connection: {:?}", &err);
            HeaderMap::new()
        }
    };

    Ok((ips, hddrs))
}

#[test]
fn test_look_and_connect2() {
    let mut site = Site { host: "yahoo.com".to_owned(), addrs: Vec::new(), headers: HeaderMap::new(), bigip: None };

    (site.addrs, site.headers) = look_and_connect2(&site).unwrap();
    
    assert!(site.addrs.len() > 0);
    assert!(site.headers.len() > 0);
}

// bigip_by_header takes a HeaderMap and returns a bool indicating whether the
// `server` header contains the case insensitive string "bigip" or not.
pub fn bigip_by_header(headers: &HeaderMap) -> bool {
    match headers.get("server") {
        Some(val) => {
            val.to_str().unwrap().to_lowercase().contains("bigip")
        },
        None => false
    }
}

#[test]
fn test_bigip_by_header() {
    let test_cases = vec!["bigip", "BIGIP", "BigIP", "BiGiP"];
    for test_case in test_cases {
        let mut headers = HeaderMap::new();
        headers.insert("server", HeaderValue::from_static(test_case));
        assert!(bigip_by_header(&headers));
    }

    let mut mixed_headers = HeaderMap::new();
    mixed_headers.insert("location", HeaderValue::from_static("www.example.com"));
    mixed_headers.insert("server", HeaderValue::from_static("bigip"));
    mixed_headers.insert("connection", HeaderValue::from_static("Keep-Alive"));
    mixed_headers.insert("content-length", HeaderValue::from_static("0"));
    assert!(bigip_by_header(&mixed_headers));

    let mut failed_headers = HeaderMap::new();
    failed_headers.insert("location", HeaderValue::from_static("www.example.com"));
    failed_headers.insert("server", HeaderValue::from_static("nginx/1.2.3"));
    failed_headers.insert("connection", HeaderValue::from_static("Keep-Alive"));
    failed_headers.insert("content-length", HeaderValue::from_static("0"));
    assert!(!bigip_by_header(&failed_headers));
}

// bigip_by_ip takes a vector of IpAddr and a vector of Ipv4Nets and returns a bool
// indicating whether any of the IP addresses matches any of the subnets.
pub fn bigip_by_ip(ips: &[IpAddr], subnets: &[Ipv4Net]) -> bool {
    for ip in ips.iter() {
        for subnet in subnets.iter() {
            if let IpAddr::V4(v4) = ip {
                if subnet.contains(v4) {
                    return true
                }
            };
        }
    }
    false
}

#[test]
fn test_bigip_by_ip() {
    // TODO
    let mut ips = Vec::new();
    ips.push(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)));
    ips.push(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));

    let mut subnets = Vec::new();
    subnets.push("172.16.0.0/24".parse().unwrap());
    subnets.push("192.168.0.0/24".parse().unwrap());

    assert!(bigip_by_ip(&ips, &subnets));
}
