/********************************************************************************
 * Copyright (c) 2023 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::str::FromStr;

use regex::Regex;

use crate::uprotocol::uri::UUri;
use crate::uprotocol::{UAuthority, UEntity, UResource};
use crate::uri::validator::UriValidator;

use crate::uri::serializer::{MicroUriSerializer, UriSerializer};

#[derive(Debug, PartialEq)]
pub struct SerializationError {
    message: String,
}

impl SerializationError {
    pub fn new<T>(message: T) -> SerializationError
    where
        T: Into<String>,
    {
        SerializationError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SerializationError {}

impl TryFrom<&UUri> for String {
    type Error = SerializationError;

    /// Attempts to serialize a `UUri` into a `String`.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `UUri` to be converted into a `String`.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the `String` representation of the URI or a `SerializationError`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::uprotocol::{UAuthority, UEntity, UResource};
    /// use up_rust::uprotocol::UUri;
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         name: "example.com".to_string(),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         name: "rpc".to_string(),
    ///         instance: Some("raise".to_string()),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     authority: None.into(),
    ///     ..Default::default()
    /// };
    ///
    /// let uri_from = String::try_from(&uri).unwrap();
    /// assert_eq!("/example.com//rpc.raise", uri_from);
    /// ````
    fn try_from(uri: &UUri) -> Result<Self, Self::Error> {
        if UriValidator::is_empty(uri) {
            return Err(SerializationError::new("URI is empty"));
        }

        let mut output = String::default();
        if let Some(authority) = uri.authority.as_ref() {
            output.push_str(UUri::build_authority_part_of_uri(authority).as_str());
        }
        output.push('/');
        if let Some(entity) = uri.entity.as_ref() {
            output.push_str(UUri::build_entity_part_of_uri(entity).as_str());
        }
        output.push_str(UUri::build_resource_part_of_uri(uri).as_str());

        // remove trailing slashes
        Ok(Regex::new(r"/+$")
            .unwrap()
            .replace_all(&output, "")
            .into_owned())
    }
}

impl FromStr for UUri {
    type Err = SerializationError;

    /// Attempts to serialize a `String` into a `UUri`.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `String` to be converted into a `UUri`.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the `UUri` representation of the URI or a `SerializationError`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::str::FromStr;
    /// use up_rust::uprotocol::{UAuthority, UEntity, UResource};
    /// use up_rust::uprotocol::UUri;
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         name: "example.com".to_string(),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         name: "rpc".to_string(),
    ///         instance: Some("raise".to_string()),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     authority: None.into(),
    ///     ..Default::default()
    /// };
    ///
    /// let uri_from = UUri::from_str("/example.com//rpc.raise").unwrap();
    /// assert_eq!(uri, uri_from);
    /// ````
    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        if uri.is_empty() {
            return Err(SerializationError::new("URI is empty"));
        }

        // strip leading scheme definition (`up`) up to and including `:`
        let uri = if let Some(index) = uri.find(':') {
            uri[index + 1..].to_string()
        } else {
            uri.replace('\\', "/")
        };
        let is_local: bool = !uri.starts_with("//");
        let uri_parts = Self::pattern_split(&uri, "/");

        if uri_parts.len() < 2 {
            return Err(SerializationError::new("URI missing UEntity or UResource"));
        }

        #[allow(unused_assignments)]
        let mut name: String = String::default();
        let mut version: String = String::default();
        let mut resource: Option<UResource> = None;
        let mut authority: Option<UAuthority> = None;

        if is_local {
            name = uri_parts[1].to_string();
            if uri_parts.len() > 2 {
                version = uri_parts[2].to_string();
            }
            if uri_parts.len() > 3 {
                resource = Some(UResource::from(uri_parts[3].as_str()));
            }
        } else {
            if uri_parts.len() > 2 {
                if uri_parts[2].trim().is_empty() {
                    return Err(SerializationError::new("Remote URI missing UAuthority"));
                }
                authority = Some(UAuthority {
                    name: Some(uri_parts[2].clone()),
                    ..Default::default()
                });
            }
            if uri_parts.len() > 3 {
                name = uri_parts[3].to_string();
                if uri_parts.len() > 4 {
                    version = uri_parts[4].to_string();
                }
                if uri_parts.len() > 5 {
                    resource = Some(UResource::from(uri_parts[5].as_str()));
                }
            } else {
                return Ok(UUri {
                    authority: authority.into(),
                    ..Default::default()
                });
            }
        }

        // Compatibility note: in the Java SDK, UEntity versions are 'int', therefore default to 0. For some reason,
        // UUris with a 0 version, the version is not properly serialized back to a string (0 is omitted). Anyways,
        // we handle this properly. There either is a version, or there is not.
        let mut ve: Option<u32> = None;
        if !version.is_empty() {
            if let Ok(version) = version.parse::<u32>() {
                ve = Some(version);
            } else {
                return Err(SerializationError::new(format!(
                    "Could not parse version number - expected an unsigned integer, got {}",
                    version
                )));
            }
        }

        let entity = UEntity {
            name,
            version_major: ve,
            ..Default::default()
        };

        Ok(UUri {
            entity: Some(entity).into(),
            authority: authority.into(),
            resource: resource.into(),
            ..Default::default()
        })
    }
}

impl TryFrom<String> for UUri {
    type Error = SerializationError;

    /// Attempts to serialize a `String` into a `UUri`.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `String` to be converted into a `UUri`.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the `UUri` representation of the URI or a `SerializationError`.
    fn try_from(uri: String) -> Result<Self, Self::Error> {
        UUri::from_str(uri.as_str())
    }
}

impl TryFrom<&str> for UUri {
    type Error = SerializationError;

    /// Attempts to serialize a `String` into a `UUri`.
    ///
    /// # Arguments
    ///
    /// * `uri` - The `String` to be converted into a `UUri`.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the `UUri` representation of the URI or a `SerializationError`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::uprotocol::{UAuthority, UEntity, UResource};
    /// use up_rust::uprotocol::UUri;
    ///
    /// let uri = UUri {
    ///     entity: Some(UEntity {
    ///         name: "example.com".to_string(),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     resource: Some(UResource {
    ///         name: "rpc".to_string(),
    ///         instance: Some("raise".to_string()),
    ///         ..Default::default()
    ///     })
    ///     .into(),
    ///     authority: None.into(),
    ///     ..Default::default()
    /// };
    ///
    /// let uri_from = UUri::try_from("/example.com//rpc.raise").unwrap();
    /// assert_eq!(uri, uri_from);
    /// ````
    fn try_from(uri: &str) -> Result<Self, Self::Error> {
        UUri::from_str(uri)
    }
}

impl TryFrom<UUri> for Vec<u8> {
    type Error = SerializationError;

    fn try_from(value: UUri) -> Result<Self, Self::Error> {
        MicroUriSerializer::serialize(&value).map_err(|e| SerializationError::new(e.to_string()))
    }
}

impl TryFrom<Vec<u8>> for UUri {
    type Error = SerializationError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        MicroUriSerializer::deserialize(value).map_err(|e| SerializationError::new(e.to_string()))
    }
}

impl UUri {
    /// Builds a fully resolved `UUri` from the serialized long format and the serialized micro format.
    ///
    /// # Arguments
    /// * `long_uri` - uri serialized as a string.
    /// * `micro_uri` - uri serialized as a byte slice.
    ///
    /// # Returns
    /// If successful, returns an UUri object serialized from the input formats. Returns `SerializationError` if either of the input uris
    /// are empty, in case the deserialization fails, or if the resulting uri cannot be resolved.
    pub fn build_resolved(long_uri: &str, micro_uri: &[u8]) -> Result<UUri, SerializationError> {
        if long_uri.is_empty() {
            return Err(SerializationError::new("Long URI is empty"));
        }
        if micro_uri.is_empty() {
            return Err(SerializationError::new("Micro URI is empty"));
        }

        let long_uri_parsed = UUri::from_str(long_uri)?;
        let micro_uri_parsed = UUri::try_from(micro_uri.to_vec())?;

        let mut auth = match micro_uri_parsed.authority.into_option() {
            Some(value) => value,
            None => return Err(SerializationError::new("Micro URI is missing UAuthority")),
        };
        let mut ue = match micro_uri_parsed.entity.into_option() {
            Some(value) => value,
            None => return Err(SerializationError::new("Micro URI is missing UEntity")),
        };
        let mut ure = match long_uri_parsed.resource.into_option() {
            Some(value) => value,
            None => return Err(SerializationError::new("Long URI is missing UResource")),
        };

        if let Some(authority) = long_uri_parsed.authority.as_ref() {
            if let Some(name) = authority.get_name() {
                auth.name = Some(name.to_owned());
            }
        }
        if let Some(entity) = long_uri_parsed.entity.as_ref() {
            ue.name = entity.name.clone();
        }
        if let Some(resource) = micro_uri_parsed.resource.as_ref() {
            ure.id = resource.id;
        }

        let uri = UUri {
            authority: Some(auth).into(),
            entity: Some(ue).into(),
            resource: Some(ure).into(),
            ..Default::default()
        };

        if UriValidator::is_resolved(&uri) {
            Ok(uri)
        } else {
            Err(SerializationError::new(format!(
                "Could not resolve uri {:?}",
                uri
            )))
        }
    }

    /// Creates the resrouce part of the uProtocol URI from a `UUri` object representing a service or an application.
    ///
    /// # Parameters
    ///
    /// - `uri`: A `UURi` object that represents a service or an application.
    ///
    /// # Returns
    ///
    /// Returns a `String` representing the resource part of the uProtocol URI.
    fn build_resource_part_of_uri(uri: &UUri) -> String {
        let mut output = String::default();

        if let Some(resource) = uri.resource.as_ref() {
            output.push('/');
            output.push_str(&resource.name);

            if let Some(instance) = &resource.instance {
                output.push('.');
                output.push_str(instance);
            }
            if let Some(message) = &resource.message {
                output.push('#');
                output.push_str(message);
            }
        }

        output
    }

    /// Creates the service part of the uProtocol URI from a `UEntity` object representing a service or an application.
    ///
    /// # Parameters
    ///
    /// - `entity`: A `UEntity` object that represents a service or an application.
    ///
    /// # Returns
    ///
    /// Returns a `String` representing the service part of the uProtocol URI.
    fn build_entity_part_of_uri(entity: &UEntity) -> String {
        let mut output = String::from(entity.name.trim());
        output.push('/');

        if let Some(version) = entity.version_major {
            output.push_str(&version.to_string());
        }

        output
    }

    /// Creates the authority part of the uProtocol URI from an authority object.
    ///
    /// # Arguments
    /// * `authority` - Represents the deployment location of a specific Software Entity.
    ///
    /// # Returns
    /// Returns the `String` representation of the `Authority` in the uProtocol URI.
    fn build_authority_part_of_uri(authority: &UAuthority) -> String {
        let mut output = String::from("//");
        if let Some(name) = authority.name.as_ref() {
            output.push_str(name.as_str());
        }
        output
    }

    fn pattern_split(input: &str, pattern: &str) -> Vec<String> {
        let mut result: Vec<String> = input
            .split(pattern)
            .map(std::string::ToString::to_string)
            .collect();

        // Remove trailing empty strings
        while let Some(last) = result.last() {
            if last.is_empty() {
                result.pop();
            } else {
                break;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(""; "fail for empty string")]
    #[test_case("/"; "fail for schema and slash")]
    #[test_case("//"; "fail for schema and double slash")]
    #[test_case("///body.access"; "fail for schema and 3 slash and content")]
    #[test_case("////body.access"; "fail for schema and 4 slash and content")]
    #[test_case("/////body.access"; "fail for schema and 5 slash and content")]
    #[test_case("//////body.access"; "fail for schema and 6 slash and content")]
    fn test_try_from_string_fail(string: &str) {
        let parsing_result = UUri::from_str(string);
        assert!(parsing_result.is_err());
    }

    #[test_case(UUri::default(); "fail for default uri")]
    fn test_try_from_uri_fail(uri: UUri) {
        let parsing_result = String::try_from(&uri);
        assert!(parsing_result.is_err());
    }

    #[test_case("/body.access",
        UUri { entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(), ..Default::default() };
        "succeed for local service")]
    #[test_case("/body.access/1",
        UUri { entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(), ..Default::default() };
        "succeed for local service with version")]
    #[test_case("/body.access//door",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with resource name")]
    #[test_case("/body.access/1/door",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with version with resource name")]
    #[test_case("/body.access//door.front_left",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with resource name with instance")]
    #[test_case("/body.access/1/door.front_left",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with version with resource name with instance")]
    #[test_case("/body.access//door.front_left#Door",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with resource name with instance with message")]
    #[test_case("/body.access/1/door.front_left#Door",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local service with version with resource name with instance with message")]
    #[test_case("/exampleapp//rpc.response",
        UUri {
            entity: Some(UEntity { name: "exampleapp".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "rpc".to_string(), instance: Some("response".to_string()), id: Some(0),  ..Default::default() }).into(),    // id is '0' for rpc repsonses
            ..Default::default()
        };
        "succeed for local rpc service uri")]
    #[test_case("/exampleapp/1/rpc.response",
        UUri {
            entity: Some(UEntity { name: "exampleapp".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "rpc".to_string(), instance: Some("response".to_string()), id: Some(0),  ..Default::default() }).into(),    // id is '0' for rpc repsonses
            ..Default::default()
        };
        "succeed for local rpc service uri with version")]
    #[test_case("//VCU.MY_CAR_VIN",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            ..Default::default()
        };
        "succeed for remote uri")]
    #[test_case("//VCU.MY_CAR_VIN/body.access",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            ..Default::default()
        };
        "succeed for remote uri with service")]
    #[test_case("//VCU.MY_CAR_VIN/body.access/1",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with version")]
    #[test_case("//VCU.MY_CAR_VIN/body.access//door",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with resource name")]
    #[test_case("//VCU.MY_CAR_VIN/body.access//door.front_left",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with resource name with instance")]
    #[test_case("//VCU.MY_CAR_VIN/body.access/1/door.front_left",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with version with resource name with instance")]
    #[test_case("//VCU.MY_CAR_VIN/body.access//door.front_left#Door",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with resource name with instance with message")]
    #[test_case("//VCU.MY_CAR_VIN/body.access/1/door.front_left#Door",
        UUri {
            authority: Some(UAuthority { name: Some("VCU.MY_CAR_VIN".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with service with version with resource name with instance with message")]
    #[test_case("//example.cloud/exampleapp//rpc.response",
        UUri {
            authority: Some(UAuthority { name: Some("example.cloud".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "exampleapp".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "rpc".to_string(), instance: Some("response".to_string()), id: Some(0),  ..Default::default() }).into(),    // id is '0' for rpc repsonses
            ..Default::default()
        };
        "succeed for remote rpc uri with service")]
    #[test_case("//example.cloud/exampleapp/1/rpc.response",
        UUri {
            authority: Some(UAuthority { name: Some("example.cloud".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "exampleapp".to_string(), version_major: Some(1), ..Default::default()}).into(),
            resource: Some(UResource { name: "rpc".to_string(), instance: Some("response".to_string()), id: Some(0),  ..Default::default() }).into(),    // id is '0' for rpc repsonses
            ..Default::default()
        };
        "succeed for remote rpc uri with service with version")]
    fn test_try_from_success(string: &str, expected_uri: UUri) {
        let parsing_result = UUri::from_str(string);
        assert!(parsing_result.is_ok());
        let parsed_uri = parsing_result.unwrap();
        assert_eq!(expected_uri, parsed_uri);

        let parsing_result = String::try_from(&parsed_uri);
        assert!(parsing_result.is_ok());
        assert_eq!(string, parsing_result.unwrap());
    }

    #[test_case("custom:/body.access//door.front_left#Door",
        UUri {
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for local uri with custom scheme with service with resource name with instance with message")]
    #[test_case("custom://vcu.vin/body.access//door.front_left#Door",
        UUri {
            authority: Some(UAuthority { name: Some("vcu.vin".to_string()), ..Default::default()}).into(),
            entity: Some(UEntity { name: "body.access".to_string(), ..Default::default()}).into(),
            resource: Some(UResource { name: "door".to_string(), instance: Some("front_left".to_string()), message: Some("Door".to_string()), ..Default::default() }).into(),
            ..Default::default()
        };
        "succeed for remote uri with custom scheme with service with resource name with instance with message")]
    fn test_try_from_custom_scheme_success(string: &str, expected_uri: UUri) {
        let parsing_result = UUri::from_str(string);
        assert!(parsing_result.is_ok());
        let parsed_uri = parsing_result.unwrap();
        assert_eq!(expected_uri, parsed_uri);

        let string = string.split_once(':').unwrap().1; // remove prefix up to and including ':' from the back-comparison uri, as custom schemes are ignores by UUri deserialization
        let parsing_result = String::try_from(&parsed_uri);
        assert!(parsing_result.is_ok());
        assert_eq!(string, parsing_result.unwrap());
    }

    #[test]
    fn test_build_resolved_passing_empty_long_uri_empty_micro_uri() {
        let uri: Result<UUri, SerializationError> = UUri::build_resolved("", &[]);
        assert!(uri.is_err());
    }
}
