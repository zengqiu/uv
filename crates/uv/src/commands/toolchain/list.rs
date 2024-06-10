use std::collections::BTreeSet;
use std::fmt::Write;

use anyhow::Result;

use uv_cache::Cache;
use uv_configuration::PreviewMode;
use uv_fs::Simplified;
use uv_toolchain::downloads::PythonDownloadRequest;
use uv_toolchain::{
    find_toolchains, DiscoveryError, SystemPython, Toolchain, ToolchainNotFound, ToolchainRequest,
    ToolchainSources,
};
use uv_warnings::warn_user;

use crate::commands::ExitStatus;
use crate::printer::Printer;
use crate::settings::ToolchainListIncludes;

/// List available toolchains.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn list(
    includes: ToolchainListIncludes,
    preview: PreviewMode,
    cache: &Cache,
    printer: Printer,
) -> Result<ExitStatus> {
    if preview.is_disabled() {
        warn_user!("`uv toolchain list` is experimental and may change without warning.");
    }

    let download_request = match includes {
        ToolchainListIncludes::All => Some(PythonDownloadRequest::default()),
        ToolchainListIncludes::Installed => None,
        ToolchainListIncludes::Default => Some(PythonDownloadRequest::from_env()?),
    };

    let downloads = download_request
        .as_ref()
        .map(uv_toolchain::downloads::PythonDownloadRequest::iter_downloads)
        .into_iter()
        .flatten();

    let installed = find_toolchains(
        &ToolchainRequest::Any,
        SystemPython::Required,
        &ToolchainSources::All(PreviewMode::Enabled),
        cache,
    )
    // Raise any errors encountered during discovery
    .collect::<Result<Vec<Result<Toolchain, ToolchainNotFound>>, DiscoveryError>>()?
    .into_iter()
    // Then drop any "missing" toolchains
    .filter_map(|result| match result {
        Ok(toolchain) => Some(toolchain),
        Err(_) => None,
    });

    let mut output = BTreeSet::new();
    for toolchain in installed {
        output.insert((
            toolchain.python_version().clone(),
            toolchain.key().clone(),
            Some(toolchain.interpreter().sys_executable().to_path_buf()),
        ));
    }
    for download in downloads {
        output.insert((
            download.python_version().version().clone(),
            download.key().to_owned(),
            None,
        ));
    }

    for (version, key, path) in output {
        if let Some(path) = path {
            writeln!(
                printer.stdout(),
                "{:<8} ({key}) {}",
                version.to_string(),
                path.user_display()
            )?;
        } else {
            writeln!(printer.stdout(), "{:<8} ({key})", version.to_string())?;
        }
    }

    Ok(ExitStatus::Success)
}
