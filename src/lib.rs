use std::{env, fs};

use zed_extension_api as zed;

const NAME: &str = "case-convert-lsp";
const DIR_PREFIX: &str = "case-convert-lsp-";
const DOWNLOAD_REPO: &str = "someone13574/zed-case-convert-ext";
const DOWNLOAD_TAG: &str = "v0.0.1";

fn binary_name(os: zed::Os) -> String {
    match os {
        zed::Os::Mac | zed::Os::Linux => NAME.to_string(),
        zed::Os::Windows => format!("{NAME}.exe"),
    }
}

fn asset_name(os: zed::Os, arch: zed::Architecture) -> zed::Result<String> {
    let platform = match (os, arch) {
        (zed::Os::Linux, zed::Architecture::X8664) => "linux-x64.tar.gz",
        (zed::Os::Windows, zed::Architecture::X8664) => "windows-x64.zip",
        (os, arch) => {
            return Err(format!("architecture {arch:?} not supported on os {os:?}"));
        }
    };

    Ok(format!("{NAME}-{platform}"))
}

fn asset_type(os: zed::Os) -> zed::DownloadedFileType {
    match os {
        zed::Os::Mac | zed::Os::Linux => zed::DownloadedFileType::GzipTar,
        zed::Os::Windows => zed::DownloadedFileType::Zip,
    }
}

fn download_binary(
    language_server_id: &zed::LanguageServerId,
    os: zed::Os,
    arch: zed::Architecture,
) -> zed::Result<String> {
    let version_dir = format!("{DIR_PREFIX}{DOWNLOAD_TAG}");
    let binary_path = format!("{version_dir}/{}", binary_name(os));
    if !fs::metadata(&binary_path).is_ok_and(|metadata| metadata.is_file()) {
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release =
            zed::github_release_by_tag_name(DOWNLOAD_REPO, DOWNLOAD_TAG).map_err(|err| {
                format!("failed to find release `{DOWNLOAD_TAG}` of `{DOWNLOAD_REPO}`: {err}")
            })?;
        let asset_name = asset_name(os, arch)?;
        let asset = release
            .assets
            .into_iter()
            .find(|asset| asset.name == asset_name)
            .ok_or(format!("no asset found matching `{asset_name}`"))?;

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );
        zed::download_file(&asset.download_url, &version_dir, asset_type(os))
            .map_err(|err| format!("failed to download file `{}`: {err}", asset.download_url))?;
        if os != zed::Os::Windows {
            zed::make_file_executable(&binary_path)?;
        }

        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with(DIR_PREFIX) && name != version_dir {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }
    }

    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::None,
    );

    let work_dir =
        env::current_dir().map_err(|err| format!("failed to get extension directory: {err}"))?;
    Ok(work_dir.join(binary_path).to_string_lossy().to_string())
}

struct CaseConvertExtension {
    path: Option<String>,
}

impl CaseConvertExtension {
    fn binary_path(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<String> {
        if let Some(path) = &self.path
            && fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
        {
            return Ok(path.clone());
        }

        let (os, arch) = zed::current_platform();
        let path = match worktree.which(&binary_name(os)) {
            Some(path) => path,
            None => download_binary(language_server_id, os, arch)?,
        };
        self.path = Some(path.clone());

        Ok(path)
    }
}

impl zed::Extension for CaseConvertExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self { path: None }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        Ok(zed::Command {
            command: self.binary_path(language_server_id, worktree)?,
            args: Vec::new(),
            env: Vec::new(),
        })
    }
}

zed::register_extension!(CaseConvertExtension);
