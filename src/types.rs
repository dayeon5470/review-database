use super::{Indexable, TrafficDirection};
pub use crate::account::{Account, Role};
use anyhow::Result;
use bincode::Options;
use chrono::{naive::serde::ts_nanoseconds_option, DateTime, NaiveDateTime, Utc};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cmp::Ordering, convert::TryFrom, net::IpAddr, ops::RangeInclusive};
use strum_macros::Display;

pub trait FromKeyValue: Sized {
    /// Creates a new instance from the given key and value.
    ///
    /// # Errors
    ///
    /// Returns an error if the key or value cannot be deserialized.
    fn from_key_value(key: &[u8], value: &[u8]) -> Result<Self>;
}

pub(crate) type Timestamp = i64;
pub(crate) type Source = String;
pub(crate) type Id = (Timestamp, Source);

pub struct PretrainedModel(pub Vec<u8>);

#[derive(
    Debug, Display, Copy, Clone, Eq, Hash, PartialEq, Deserialize, Serialize, PartialOrd, Ord,
)]
#[repr(u8)]
pub enum EventCategory {
    Reconnaissance = 1,
    InitialAccess,
    Execution,
    CredentialAccess,
    Discovery,
    LateralMovement,
    CommandAndControl,
    Exfiltration,
    Impact,
    HttpThreat,
}

impl TryFrom<u8> for EventCategory {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let category = match value {
            1 => Self::Reconnaissance,
            2 => Self::InitialAccess,
            3 => Self::Execution,
            4 => Self::CredentialAccess,
            5 => Self::Discovery,
            6 => Self::LateralMovement,
            7 => Self::CommandAndControl,
            8 => Self::Exfiltration,
            9 => Self::Impact,
            10 => Self::HttpThreat,
            _ => return Err("Invalid event category"),
        };
        Ok(category)
    }
}

impl From<EventCategory> for u8 {
    fn from(value: EventCategory) -> Self {
        value as Self
    }
}

#[derive(Deserialize)]
pub struct Cluster {
    pub id: i32,
    pub cluster_id: String,
    pub category_id: i32,
    pub detector_id: i32,
    pub event_ids: Vec<Timestamp>,
    pub event_sources: Vec<Source>,
    pub labels: Option<Vec<String>>,
    pub qualifier_id: i32,
    pub status_id: i32,
    pub signature: String,
    pub size: i64,
    pub score: Option<f64>,
    #[serde(with = "ts_nanoseconds_option")]
    pub last_modification_time: Option<NaiveDateTime>,
    pub model_id: i32,
}

#[derive(Deserialize, Serialize)]
pub struct Endpoint {
    pub direction: Option<TrafficDirection>,
    pub network: HostNetworkGroup,
}

// `hosts` and `networks` must be kept sorted.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct HostNetworkGroup {
    hosts: Vec<IpAddr>,
    networks: Vec<IpNet>,
    ip_ranges: Vec<RangeInclusive<IpAddr>>,
}

impl HostNetworkGroup {
    #[must_use]
    pub fn new(
        mut hosts: Vec<IpAddr>,
        mut networks: Vec<IpNet>,
        mut ip_ranges: Vec<RangeInclusive<IpAddr>>,
    ) -> Self {
        hosts.sort_unstable();
        hosts.dedup();
        networks.sort_unstable();
        networks.dedup();
        ip_ranges.sort_unstable_by(|a, b| match a.start().cmp(b.start()) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => a.end().cmp(b.end()),
            Ordering::Greater => Ordering::Greater,
        });
        ip_ranges.dedup();
        Self {
            hosts,
            networks,
            ip_ranges,
        }
    }

    #[must_use]
    pub fn hosts(&self) -> &[IpAddr] {
        &self.hosts
    }

    #[must_use]
    pub fn ip_ranges(&self) -> &[RangeInclusive<IpAddr>] {
        &self.ip_ranges
    }

    #[must_use]
    pub fn networks(&self) -> &[IpNet] {
        &self.networks
    }

    #[must_use]
    pub fn contains(&self, addr: IpAddr) -> bool {
        if self.contains_host(addr) {
            return true;
        }

        if self.networks.iter().any(|net| net.contains(&addr)) {
            return true;
        }

        if self.ip_ranges.iter().any(|range| range.contains(&addr)) {
            return true;
        }

        false
    }

    #[must_use]
    pub fn contains_host(&self, host: IpAddr) -> bool {
        self.hosts.binary_search(&host).is_ok()
    }

    #[must_use]
    pub fn contains_ip_range(&self, ip_range: &RangeInclusive<IpAddr>) -> bool {
        self.ip_ranges.contains(ip_range)
    }

    #[must_use]
    pub fn contains_network(&self, network: &IpNet) -> bool {
        self.networks.binary_search(network).is_ok()
    }
}

#[derive(Deserialize)]
pub struct Outlier {
    pub id: i32,
    #[serde(with = "serde_bytes")]
    pub raw_event: Vec<u8>,
    pub event_ids: Vec<Timestamp>,
    pub event_sources: Vec<Source>,
    pub size: i64,
    pub model_id: i32,
}

pub type SeqNo = usize;
pub type ModelScores = std::collections::HashMap<SeqNo, f64>;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct ModelBatchInfo {
    pub id: i64,
    pub earliest: i64,
    pub latest: i64,
    pub size: usize,
    pub sources: Vec<String>,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
pub enum TiCmpKind {
    IpAddress,
    Domain,
    Hostname,
    Uri,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
pub enum ValueKind {
    String,
    Integer,
    Float,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
pub enum AttrCmpKind {
    Less,
    Equal,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
    Contain,
    OpenRange,
    CloseRange,
    LeftOpenRange,
    RightOpenRange,
    NotEqual,
    NotContain,
    NotOpenRange,
    NotCloseRange,
    NotLeftOpenRange,
    NotRightOpenRange,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
pub enum ResponseKind {
    Manual,
    Blacklist,
    Whitelist,
}

#[derive(PartialEq, Deserialize, Serialize)]
pub struct Ti {
    pub ti_name: String,
    pub kind: TiCmpKind,
    pub weight: Option<f64>,
}

impl Eq for Ti {}

impl PartialOrd for Ti {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ti {
    fn cmp(&self, other: &Self) -> Ordering {
        let first = self.ti_name.cmp(&other.ti_name);
        if first != Ordering::Equal {
            return first;
        }
        let second = self.kind.cmp(&other.kind);
        if second != Ordering::Equal {
            return second;
        }
        match (self.weight, other.weight) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(s), Some(o)) => s.total_cmp(&o),
        }
    }
}

#[derive(PartialEq, Deserialize, Serialize)]
pub struct PacketAttr {
    pub attr_name: String,
    pub value_kind: ValueKind,
    pub cmp_kind: AttrCmpKind,
    pub first_value: Vec<u8>,
    pub second_value: Option<Vec<u8>>,
    pub weight: Option<f64>,
}

impl Eq for PacketAttr {}

impl PartialOrd for PacketAttr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PacketAttr {
    fn cmp(&self, other: &Self) -> Ordering {
        let first = self.attr_name.cmp(&other.attr_name);
        if first != Ordering::Equal {
            return first;
        }
        let second = self.value_kind.cmp(&other.value_kind);
        if second != Ordering::Equal {
            return second;
        }
        let third = self.cmp_kind.cmp(&other.cmp_kind);
        if third != Ordering::Equal {
            return third;
        }
        let fourth = self.first_value.cmp(&other.first_value);
        if fourth != Ordering::Equal {
            return fourth;
        }
        let fifth = self.second_value.cmp(&other.second_value);
        if fifth != Ordering::Equal {
            return fifth;
        }
        match (self.weight, other.weight) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(s), Some(o)) => s.total_cmp(&o),
        }
    }
}

#[derive(PartialEq, Deserialize, Serialize)]
pub struct Confidence {
    pub threat_category: EventCategory,
    pub threat_kind: String,
    pub confidence: f64,
    pub weight: Option<f64>,
}

impl Eq for Confidence {}

impl PartialOrd for Confidence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Confidence {
    fn cmp(&self, other: &Self) -> Ordering {
        let first = self.threat_category.cmp(&other.threat_category);
        if first != Ordering::Equal {
            return first;
        }
        let second = self.threat_kind.cmp(&other.threat_kind);
        if second != Ordering::Equal {
            return second;
        }
        let third = self.confidence.total_cmp(&other.confidence);
        if third != Ordering::Equal {
            return third;
        }
        match (self.weight, other.weight) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(s), Some(o)) => s.total_cmp(&o),
        }
    }
}

#[derive(PartialEq, Deserialize, Serialize)]
pub struct Response {
    pub minimum_score: f64,
    pub kind: ResponseKind,
}

impl Eq for Response {}

impl PartialOrd for Response {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Response {
    fn cmp(&self, other: &Self) -> Ordering {
        let first = self.minimum_score.total_cmp(&other.minimum_score);
        if first != Ordering::Equal {
            return first;
        }
        self.kind.cmp(&other.kind)
    }
}

#[derive(Deserialize, Serialize)]
pub struct TriagePolicy {
    pub id: u32,
    pub name: String,
    pub ti_db: Vec<Ti>,
    pub packet_attr: Vec<PacketAttr>,
    pub confidence: Vec<Confidence>,
    pub response: Vec<Response>,
    pub creation_time: DateTime<Utc>,
}

impl FromKeyValue for TriagePolicy {
    fn from_key_value(_key: &[u8], value: &[u8]) -> Result<Self> {
        Ok(bincode::DefaultOptions::new().deserialize(value)?)
    }
}

impl Indexable for TriagePolicy {
    fn key(&self) -> Cow<[u8]> {
        Cow::Borrowed(self.name.as_bytes())
    }
    fn index(&self) -> u32 {
        self.id
    }
    fn make_indexed_key(key: Cow<[u8]>, _index: u32) -> Cow<[u8]> {
        key
    }
    fn value(&self) -> Vec<u8> {
        bincode::DefaultOptions::new()
            .serialize(self)
            .expect("serializable")
    }

    fn set_index(&mut self, index: u32) {
        self.id = index;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Status {
    pub id: u32,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Qualifier {
    pub id: u32,
    pub description: String,
}
