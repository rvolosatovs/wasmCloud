//! wasmCloud host library

#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![forbid(clippy::unwrap_used)]

/// local lattice
pub mod local;

/// wasmbus lattice
pub mod wasmbus;

// bindle artifact fetching
pub mod bindle;

// OCI artifact fetching
pub mod oci;

// Provider archive functionality
mod par;

pub use local::{Lattice as LocalLattice, LatticeConfig as LocalLatticeConfig};
pub use wasmbus::{Lattice as WasmbusLattice, LatticeConfig as WasmbusLatticeConfig};

pub use url;

use core::num::NonZeroUsize;
use core::ops::{Deref, DerefMut};

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context as _};
use provider_archive::ProviderArchive;
use tokio::fs;
use tokio::runtime::Handle;
use tokio::sync::RwLock;
use tokio::task;
use tracing::instrument;
use url::Url;
use wascap::jwt;

#[cfg(unix)]
fn socket_pair() -> anyhow::Result<(tokio::net::UnixStream, tokio::net::UnixStream)> {
    tokio::net::UnixStream::pair().context("failed to create an unnamed unix socket pair")
}

#[cfg(windows)]
fn socket_pair() -> anyhow::Result<(tokio::io::DuplexStream, tokio::io::DuplexStream)> {
    Ok(tokio::io::duplex(8196))
}

enum ResourceRef<'a> {
    File(PathBuf),
    Bindle(&'a str),
    Oci(&'a str),
}

impl<'a> TryFrom<&'a str> for ResourceRef<'a> {
    type Error = anyhow::Error;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match Url::parse(s) {
            Ok(url) => {
                match url.scheme() {
                    "file" => url
                        .to_file_path()
                        .map(Self::File)
                        .map_err(|_| anyhow!("failed to convert `{url}` to a file path")),
                    "bindle" => s
                        .strip_prefix("bindle://")
                        .map(Self::Bindle)
                        .context("invalid Bindle reference"),
                    // TODO: Support other schemes
                    scheme => bail!("unsupported scheme `{scheme}`"),
                }
            }
            Err(url::ParseError::RelativeUrlWithoutBase) => Ok(Self::Oci(s)), // TODO: Validate
            Err(e) => {
                bail!(anyhow!(e).context("failed to parse actor reference `{actor_ref}`"))
            }
        }
    }
}

/// Fetch an actor from a reference.
#[instrument(skip(actor_ref))]
pub async fn fetch_actor(actor_ref: impl AsRef<str>) -> anyhow::Result<Vec<u8>> {
    let actor_ref = actor_ref.as_ref();
    match Url::parse(actor_ref) {
        Ok(url) => {
            match url.scheme() {
                "file" => {
                    let path = url
                        .to_file_path()
                        .map_err(|_| anyhow!("failed to convert `{url}` to a file path"))?;
                    fs::read(path).await.context("failed to read actor")
                }
                "bindle" => {
                    let actor_ref = actor_ref
                        .strip_prefix("bindle://")
                        .context("invalid Bindle reference")?;
                    crate::bindle::fetch_actor(None, &actor_ref)
                        .await
                        .with_context(|| {
                            format!("failed to fetch actor under Bindle reference `{actor_ref}`")
                        })
                }
                // TODO: Support other schemes
                scheme => bail!("unsupported scheme `{scheme}`"),
            }
        }
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            // TODO: Set config
            crate::oci::fetch_actor(None, &actor_ref, true, vec![])
                .await
                .with_context(|| format!("failed to fetch actor under OCI reference `{actor_ref}`"))
        }
        Err(e) => {
            bail!(anyhow!(e).context("failed to parse actor reference `{actor_ref}`"))
        }
    }
}

///// Fetch a provider from a reference.
#[instrument(skip(provider_ref, link_name))]
pub async fn fetch_provider(
    provider_ref: impl AsRef<str>,
    link_name: impl AsRef<str>,
) -> anyhow::Result<(PathBuf, jwt::Claims<jwt::CapabilityProvider>)> {
    let provider_ref = provider_ref.as_ref();
    match Url::parse(provider_ref) {
        Ok(url) => {
            match url.scheme() {
                "file" => {
                    let path = url
                        .to_file_path()
                        .map_err(|_| anyhow!("failed to convert `{url}` to a file path"))?;
                    par::read(path, link_name)
                        .await
                        .context("failed to read provider")
                }
                "bindle" => {
                    let provider_ref = provider_ref
                        .strip_prefix("bindle://")
                        .context("invalid Bindle reference")?;
                    crate::bindle::fetch_provider(None, &provider_ref, link_name)
                        .await
                        .with_context(|| {
                            format!(
                                "failed to fetch provider under Bindle reference `{provider_ref}`"
                            )
                        })
                }
                // TODO: Support other schemes
                scheme => bail!("unsupported scheme `{scheme}`"),
            }
        }
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            // TODO: Set config
            crate::oci::fetch_provider(None, &provider_ref, true, vec![], link_name)
                .await
                .with_context(|| {
                    format!("failed to fetch provider under OCI reference `{provider_ref}`")
                })
        }
        Err(e) => {
            bail!(anyhow!(e).context("failed to parse provider reference `{provider_ref}`"))
        }
    }
}
