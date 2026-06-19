use zed_extension_api::{
    current_platform, download_file, latest_github_release, make_file_executable,
    register_extension, Architecture, Command, DownloadedFileType, Extension, GithubReleaseOptions,
    LanguageServerId, Os, Worktree,
};

const GOSU_LSP_SERVER_ID: &str = "gosu-lsp";
const GOSU_LSP_REPOSITORY: &str = "mpt83/zed-gosu";

struct GosuExtension;

impl Extension for GosuExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed_extension_api::Result<Command> {
        if language_server_id.as_ref() != GOSU_LSP_SERVER_ID {
            return Err(format!("unknown language server: {}", language_server_id.as_ref()));
        }

        if let Some(path) = worktree.which("gosu-lsp") {
            return Ok(Command {
                command: path,
                args: Vec::new(),
                env: Vec::new(),
            });
        }

        let path = install_gosu_lsp()?;
        Ok(Command {
            command: path,
            args: Vec::new(),
            env: Vec::new(),
        })
    }
}

fn install_gosu_lsp() -> zed_extension_api::Result<String> {
    let release = latest_github_release(
        GOSU_LSP_REPOSITORY,
        GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )?;

    let asset_name = gosu_lsp_asset_name()?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| {
            format!(
                "could not find asset '{}' in latest {} release",
                asset_name, GOSU_LSP_REPOSITORY
            )
        })?;

    let install_dir = format!("gosu-lsp/{}", release.version);
    download_file(
        &asset.download_url,
        &install_dir,
        DownloadedFileType::GzipTar,
    )?;

    let executable = format!("{}/gosu-lsp", install_dir);
    make_file_executable(&executable)?;
    Ok(executable)
}

fn gosu_lsp_asset_name() -> zed_extension_api::Result<String> {
    let (os, arch) = current_platform();

    let os = match os {
        Os::Mac => "apple-darwin",
        Os::Linux => "unknown-linux-gnu",
        Os::Windows => return Err("gosu-lsp does not currently ship Windows assets".to_string()),
    };

    let arch = match arch {
        Architecture::Aarch64 => "aarch64",
        Architecture::X8664 => "x86_64",
        Architecture::X86 => return Err("gosu-lsp does not currently ship x86 assets".to_string()),
    };

    Ok(format!("gosu-lsp-{}-{}.tar.gz", arch, os))
}

register_extension!(GosuExtension);
