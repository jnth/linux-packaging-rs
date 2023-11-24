// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/*! `repomd.xml` file format. */

use {
    crate::{
        error::{Result, RpmRepositoryError},
        io::ContentDigest,
    },
    serde::{Deserialize, Serialize},
    serde::{Deserializer, Serializer},
    serde::de::{Error, Visitor},
    std::io::Read,
    std::fmt::Formatter,
};

/// A `repomd.xml` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMd {
    /// Revision of the repository.
    ///
    /// Often an integer-like value.
    pub revision: String,
    /// Describes additional primary data files constituting this repository.
    pub data: Vec<RepoMdData>,
}

impl RepoMd {
    /// Construct an instance by parsing XML from a reader.
    pub fn from_reader(reader: impl Read) -> Result<Self> {
        Ok(serde_xml_rs::from_reader(reader)?)
    }

    /// Construct an instance by parsing XML from a string.
    pub fn from_xml(s: &str) -> Result<Self> {
        Ok(serde_xml_rs::from_str(s)?)
    }
}

/// A `<data>` element in a `repomd.xml` file.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepoMdData {
    /// The type of data.
    #[serde(rename = "type")]
    pub data_type: String,
    /// Content checksum of this file.
    pub checksum: Checksum,
    /// Where the file is located.
    pub location: Location,
    /// Size in bytes of the file as stored in the repository.
    pub size: Option<u64>,
    /// Time file was created/modified.
    pub timestamp: Option<Timestamp>,
    /// Content checksum of the decoded (often decompressed) file.
    #[serde(rename = "open-checksum")]
    pub open_checksum: Option<Checksum>,
    /// Size in bytes of the decoded (often decompressed) file.
    #[serde(rename = "open-size")]
    pub open_size: Option<u64>,
    /// Content checksum of header data.
    #[serde(rename = "header-checksum")]
    pub header_checksum: Option<Checksum>,
    /// Size in bytes of the header.
    #[serde(rename = "header-size")]
    pub header_size: Option<u64>,
}

/// The content checksum of a `<data>` element.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Checksum {
    /// The name of the content digest.
    #[serde(rename = "type")]
    pub name: String,
    /// The hex encoded content digest.
    #[serde(rename = "$value")]
    pub value: String,
}

impl TryFrom<Checksum> for ContentDigest {
    type Error = RpmRepositoryError;

    fn try_from(v: Checksum) -> std::result::Result<Self, Self::Error> {
        match v.name.as_str() {
            "sha1" => ContentDigest::sha1_hex(&v.value),
            "sha256" => ContentDigest::sha256_hex(&v.value),
            name => Err(RpmRepositoryError::UnknownDigestFormat(name.to_string())),
        }
    }
}

/// The location of a `<data>` element.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Location {
    pub href: String,
}

struct TimestampVisitor;

impl<'de> Visitor<'de> for TimestampVisitor {
    type Value = u64;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a positive number")
    }

    fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E> where E: Error {
        // Keep the same value
        Ok(v)
    }

    fn visit_f64<E>(self, v: f64) -> std::result::Result<Self::Value, E> where E: Error {
        use std::u64;
        // Round to the nearest `u64`
        if v < u64::MIN as f64 || v > u64::MAX as f64{
            return Err(E::custom(format!("Invalid timestamp: {}", v)))
        }
        Ok(v.round() as u64)
    }
}

/// Time file was created/modified.
#[derive(Clone, Debug, PartialEq)]
pub struct Timestamp(u64);

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error> where D: Deserializer<'de> {
        Ok(Self(deserializer.deserialize_f64(TimestampVisitor)?))
    }
}


#[cfg(test)]
mod test {
    use super::*;

    const FEDORA_35_REPOMD_XML: &str = include_str!("../testdata/fedora-35-repodata.xml");
    const WITH_FLOAT_TIMESTAMP: &str = include_str!("../testdata/with-float-timestamp.xml");

    #[test]
    fn fedora_35_parse() -> Result<()> {
        let result = RepoMd::from_xml(FEDORA_35_REPOMD_XML)?;
        assert_eq!(result.data[0].timestamp, Some(Timestamp(1635225121)));

        Ok(())
    }

    #[test]
    fn with_float_timestamp_parse() -> Result<()> {
        let result = RepoMd::from_xml(WITH_FLOAT_TIMESTAMP)?;
        assert_eq!(result.data[0].timestamp, Some(Timestamp(1635225122)));

        Ok(())
    }
}
