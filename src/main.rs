use self_update::cargo_crate_version;

const OWNER: &str = "Wimt";
const REPO_NAME: &str = "CanadaLauncher";

fn main() -> Result<(), Box<dyn (::std::error::Error)>> {
    println!("Version 0.1.1");
    let status = self_update::backends::github::Update::configure()
        .repo_owner(OWNER)
        .repo_name(REPO_NAME)
        .bin_name("CanadaLauncher")
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;
    println!("Update status: `{}`!", status.version());
    Ok(())
}
