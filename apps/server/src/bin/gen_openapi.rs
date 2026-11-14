use std::path::PathBuf;

/// Repo-relative spec path, kept in one place: the generator writes it, the
/// contracts package reads it, `make contract-check` diffs it.
const SPEC: &str = "packages/contracts/openapi.json";

fn main() -> std::process::ExitCode {
    let Ok(json) = leaflet::openapi::spec().to_pretty_json() else {
        eprintln!("cannot serialize the openapi document");
        return std::process::ExitCode::FAILURE;
    };

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(SPEC);

    if let Some(dir) = out.parent()
        && let Err(err) = std::fs::create_dir_all(dir)
    {
        eprintln!("cannot create {}: {err}", dir.display());
        return std::process::ExitCode::FAILURE;
    }
    match std::fs::write(&out, format!("{json}\n")) {
        Ok(()) => {
            println!("wrote {}", out.display());
            std::process::ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("cannot write {}: {err}", out.display());
            std::process::ExitCode::FAILURE
        }
    }
}
