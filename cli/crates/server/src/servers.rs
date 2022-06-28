use crate::consts::{
    EPHEMERAL_PORT_RANGE, GIT_IGNORE_CONTENTS, GIT_IGNORE_FILE, MIN_NODE_VERSION, SCHEMA_PARSER_DIR,
    SCHEMA_PARSER_INDEX, WORKER_DIR, WORKER_FOLDER_VERSION_FILE,
};
use crate::types::{Assets, ServerMessage};
use crate::{bridge, errors::ServerError};
use common::environment::Environment;
use common::types::LocalAddressType;
use common::utils::find_available_port_in_range;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::{
    fs,
    process::Stdio,
    thread::{self, JoinHandle},
};
use tokio::process::Command;
use tokio::runtime::Builder;
use tokio::sync::Notify;
use version_compare::Version;
use which::which;

/// starts a development server by unpacking any files needed by the gateway worker
/// and starting the miniflare cli in `user_grafbase_path` in [`Environment`]
///
/// # Errors
///
/// returns [`ServerError::ReadVersion`] if the version file for the extracted worker files cannot be read
///
/// returns [`ServerError::CreateDir`] if the `WORKER_DIR` cannot be created
///
/// returns [`ServerError::WriteFile`] if a file cannot be written into `WORKER_DIR`
///
/// # Panics
///
/// The spawned server and miniflare thread can panic if either of the two inner spawned threads panic
#[must_use]
pub fn start(port: u16) -> (JoinHandle<Result<(), ServerError>>, Receiver<ServerMessage>) {
    let (sender, receiver): (Sender<ServerMessage>, Receiver<ServerMessage>) = mpsc::channel();

    let handle = thread::spawn(move || {
        export_embedded_files()?;

        create_project_dot_grafbase_folder()?;

        // the bridge runs on an available port within the ephemeral port range which is also supplied to the worker,
        // making the port choice and availability transprent to the user
        let bridge_port = find_available_port_in_range(EPHEMERAL_PORT_RANGE, LocalAddressType::Localhost)
            .ok_or(ServerError::AvailablePort)?;

        // manual implementation of #[tokio::main] due to a rust analyzer issue
        Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { spawn_servers(port, bridge_port, sender).await })
    });
    (handle, receiver)
}

#[tracing::instrument(level = "trace")]
async fn spawn_servers(worker_port: u16, bridge_port: u16, sender: Sender<ServerMessage>) -> Result<(), ServerError> {
    validate_dependencies().await?;

    run_schema_parser().await?;

    let environment = Environment::get();

    let bridge_ready_sender = Arc::new(Notify::new());
    let bridge_ready_receiver = bridge_ready_sender.clone();

    let bridge_handle = tokio::spawn(async move { bridge::start(bridge_port, bridge_ready_sender).await });

    trace!("waiting for bridge ready");

    bridge_ready_receiver.notified().await;

    trace!("bridge ready");

    let registry_path = environment
        .project_grafbase_registry_path
        .to_str()
        .ok_or(ServerError::ProjectPath)?;

    trace!("spawining miniflare");

    let spawned = Command::new("node")
        .args(&[
            // used by miniflare when running normally as well
            "--experimental-vm-modules",
            "./node_modules/miniflare/dist/src/cli.js",
            "--host",
            "127.0.0.1",
            "--port",
            &worker_port.to_string(),
            "--no-update-check",
            "--no-cf-fetch",
            "--wrangler-config",
            "wrangler.toml",
            "--binding",
            &format!("BRIDGE_PORT={bridge_port}"),
            "--text-blob",
            &format!("REGISTRY={registry_path}"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&environment.user_dot_grafbase_path)
        .spawn()
        .map_err(ServerError::MiniflareCommandError)?;

    sender
        .send(ServerMessage::Ready(worker_port))
        .expect("cannot send message");

    let output = spawned
        .wait_with_output()
        .await
        .map_err(ServerError::MiniflareCommandError)?;

    output
        .status
        .success()
        .then(|| {})
        .ok_or_else(|| ServerError::MiniflareError(String::from_utf8_lossy(&output.stderr).into_owned()))?;

    bridge_handle.await??;

    Ok(())
}

fn export_embedded_files() -> Result<(), ServerError> {
    let environment = Environment::get();

    let worker_path = environment.user_dot_grafbase_path.join(WORKER_DIR);

    // CARGO_PKG_VERSION is guaranteed be valid semver
    let current_version = Version::from(env!("CARGO_PKG_VERSION")).unwrap();

    let worker_version_path = worker_path.join(WORKER_FOLDER_VERSION_FILE);

    let export_files = if worker_path.is_dir() {
        let worker_version = fs::read_to_string(&worker_version_path).map_err(|_| ServerError::ReadVersion)?;

        // derived from CARGO_PKG_VERSION, guaranteed be valid semver
        current_version > Version::from(&worker_version).unwrap()
    } else {
        true
    };

    if export_files {
        trace!("writing worker files");

        fs::create_dir_all(&environment.user_dot_grafbase_path).map_err(|_| ServerError::CreateCacheDir)?;

        let gitignore_path = &environment.user_dot_grafbase_path.join(GIT_IGNORE_FILE);

        fs::write(gitignore_path, GIT_IGNORE_CONTENTS)
            .map_err(|_| ServerError::WriteFile(gitignore_path.to_string_lossy().into_owned()))?;

        let mut write_results = Assets::iter().map(|path| {
            let file = Assets::get(path.as_ref());

            let full_path = environment.user_dot_grafbase_path.join(path.as_ref());

            let parent = full_path.parent().expect("must have a parent");

            let parent_exists = parent.metadata().is_ok();

            let create_dir_result = if parent_exists {
                Ok(())
            } else {
                fs::create_dir_all(&parent)
            };

            // must be Some(file) since we're iterating over existing paths
            let write_result = create_dir_result.and_then(|_| fs::write(&full_path, file.unwrap().data));

            (write_result, full_path)
        });

        if let Some((_, path)) = write_results.find(|(result, _)| result.is_err()) {
            let error_path_string = path.to_string_lossy().into_owned();
            return Err(ServerError::WriteFile(error_path_string));
        }

        if fs::write(&worker_version_path, current_version.as_str()).is_err() {
            let worker_version_path_string = worker_version_path.to_string_lossy().into_owned();
            return Err(ServerError::WriteFile(worker_version_path_string));
        };
    }

    Ok(())
}

fn create_project_dot_grafbase_folder() -> Result<(), ServerError> {
    let environment = Environment::get();

    let project_dot_grafbase_path = environment.project_dot_grafbase_path.clone();

    if fs::metadata(&project_dot_grafbase_path).is_err() {
        trace!("creating .grafbase directory");
        fs::create_dir_all(&project_dot_grafbase_path).map_err(|_| ServerError::CreateCacheDir)?;
        fs::write(&project_dot_grafbase_path.join(GIT_IGNORE_FILE), "*\n").map_err(|_| ServerError::CreateCacheDir)?;
    }

    Ok(())
}

// schema-parser is run via NodeJS due to it being built to run in a Wasm (via wasm-bindgen) environement
// and due to schema-parser not being open source
async fn run_schema_parser() -> Result<(), ServerError> {
    trace!("parsing schema");

    let environment = Environment::get();

    let parser_path = environment
        .user_dot_grafbase_path
        .join(SCHEMA_PARSER_DIR)
        .join(SCHEMA_PARSER_INDEX);

    let output = Command::new("node")
        .args(&[
            &parser_path.to_str().ok_or(ServerError::CachePath)?,
            &environment
                .project_grafbase_schema_path
                .to_str()
                .ok_or(ServerError::ProjectPath)?,
        ])
        .current_dir(&environment.project_dot_grafbase_path)
        .stderr(Stdio::piped())
        .spawn()
        .map_err(ServerError::SchemaParserError)?
        .wait_with_output()
        .await
        .map_err(ServerError::SchemaParserError)?;

    output
        .status
        .success()
        .then(|| {})
        .ok_or_else(|| ServerError::ParseSchema(String::from_utf8_lossy(&output.stderr).into_owned()))?;

    Ok(())
}

async fn get_node_version_string() -> Result<String, ServerError> {
    let output = Command::new("node")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| ServerError::CheckNodeVersion)?
        .wait_with_output()
        .await
        .map_err(|_| ServerError::CheckNodeVersion)?;

    let node_version_string = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    Ok(node_version_string)
}

async fn validate_node_version() -> Result<(), ServerError> {
    trace!("validating Node.js version");
    trace!("minimal supported Node.js version: {}", MIN_NODE_VERSION);

    let node_version_string = get_node_version_string().await?;

    trace!("installed node version: {}", node_version_string);

    let node_version = Version::from(&node_version_string).ok_or(ServerError::CheckNodeVersion)?;
    let min_version = Version::from(MIN_NODE_VERSION).expect("must be valid");

    if node_version >= min_version {
        Ok(())
    } else {
        Err(ServerError::OutdatedNode(
            node_version_string,
            MIN_NODE_VERSION.to_owned(),
        ))
    }
}

async fn validate_dependencies() -> Result<(), ServerError> {
    trace!("validating dependencies");

    which("node").map_err(|_| ServerError::NodeInPath)?;

    validate_node_version().await?;

    Ok(())
}
