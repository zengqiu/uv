use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use tracing::{instrument, warn};

use distribution_filename::DistFilename;
use distribution_types::{Dist, IndexUrl, PrioritizedDistribution, ResolvableDist};
use platform_tags::Tags;
use puffin_client::{FlatIndex, SimpleMetadata};
use puffin_normalize::PackageName;
use puffin_warnings::warn_user_once;
use pypi_types::{Hashes, Yanked};

use crate::pubgrub::PubGrubVersion;
use crate::python_requirement::PythonRequirement;
use crate::yanks::AllowedYanks;

/// A map from versions to distributions.
#[derive(Debug, Default, Clone)]
pub struct VersionMap(BTreeMap<PubGrubVersion, PrioritizedDistribution>);

impl VersionMap {
    /// Initialize a [`VersionMap`] from the given metadata.
    #[instrument(skip_all, fields(package_name))]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_metadata(
        metadata: SimpleMetadata,
        package_name: &PackageName,
        index: &IndexUrl,
        tags: &Tags,
        python_requirement: &PythonRequirement,
        allowed_yanks: &AllowedYanks,
        exclude_newer: Option<&DateTime<Utc>>,
        flat_index: Option<FlatIndex<PubGrubVersion>>,
    ) -> Self {
        // If we have packages of the same name from find links, gives them priority, otherwise start empty
        let mut version_map: BTreeMap<PubGrubVersion, PrioritizedDistribution> =
            flat_index.map(|overrides| overrides.0).unwrap_or_default();

        // Collect compatible distributions.
        for (version, files) in metadata {
            for (filename, file) in files.all() {
                // Support resolving as if it were an earlier timestamp, at least as long files have
                // upload time information.
                if let Some(exclude_newer) = exclude_newer {
                    match file.upload_time.as_ref() {
                        Some(upload_time) if upload_time >= exclude_newer => {
                            continue;
                        }
                        None => {
                            warn_user_once!(
                                "{} is missing an upload date, but user provided: {exclude_newer}",
                                file.filename,
                            );
                            continue;
                        }
                        _ => {}
                    }
                }

                // When resolving, exclude yanked files.
                if file.yanked.as_ref().is_some_and(Yanked::is_yanked) {
                    if allowed_yanks.allowed(package_name, &version) {
                        warn!("Allowing yanked version: {}", file.filename);
                    } else {
                        continue;
                    }
                }

                // Prioritize amongst all available files.
                let requires_python = file.requires_python.clone();
                let hash = file.hashes.clone();
                match filename {
                    DistFilename::WheelFilename(filename) => {
                        // To be compatible, the wheel must both have compatible tags _and_ have a
                        // compatible Python requirement.
                        let priority = filename.compatibility(tags).filter(|_| {
                            file.requires_python
                                .as_ref()
                                .map_or(true, |requires_python| {
                                    requires_python.contains(python_requirement.target())
                                })
                        });
                        let dist = Dist::from_registry(
                            DistFilename::WheelFilename(filename),
                            file,
                            index.clone(),
                        );
                        match version_map.entry(version.clone().into()) {
                            Entry::Occupied(mut entry) => {
                                entry.get_mut().insert_built(
                                    dist,
                                    requires_python,
                                    Some(hash),
                                    priority,
                                );
                            }
                            Entry::Vacant(entry) => {
                                entry.insert(PrioritizedDistribution::from_built(
                                    dist,
                                    requires_python,
                                    Some(hash),
                                    priority,
                                ));
                            }
                        }
                    }
                    DistFilename::SourceDistFilename(filename) => {
                        let dist = Dist::from_registry(
                            DistFilename::SourceDistFilename(filename),
                            file,
                            index.clone(),
                        );
                        match version_map.entry(version.clone().into()) {
                            Entry::Occupied(mut entry) => {
                                entry
                                    .get_mut()
                                    .insert_source(dist, requires_python, Some(hash));
                            }
                            Entry::Vacant(entry) => {
                                entry.insert(PrioritizedDistribution::from_source(
                                    dist,
                                    requires_python,
                                    Some(hash),
                                ));
                            }
                        }
                    }
                }
            }
        }

        Self(version_map)
    }

    /// Return the [`DistFile`] for the given version, if any.
    pub(crate) fn get(&self, version: &PubGrubVersion) -> Option<ResolvableDist> {
        self.0.get(version).and_then(PrioritizedDistribution::get)
    }

    /// Return an iterator over the versions and distributions.
    pub(crate) fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&PubGrubVersion, ResolvableDist)> {
        self.0
            .iter()
            .filter_map(|(version, dist)| Some((version, dist.get()?)))
    }

    /// Return the [`Hashes`] for the given version, if any.
    pub(crate) fn hashes(&self, version: &PubGrubVersion) -> Vec<Hashes> {
        self.0
            .get(version)
            .map(|file| file.hashes().to_vec())
            .unwrap_or_default()
    }
}

impl From<FlatIndex<PubGrubVersion>> for VersionMap {
    fn from(flat_index: FlatIndex<PubGrubVersion>) -> Self {
        Self(flat_index.0)
    }
}
