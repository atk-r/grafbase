use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use hyper::Error as HyperError;
use notify::Error as NotifyError;
use serde_json::json;
use sqlx::Error as SqlxError;
use std::io::Error as IoError;
use std::path::PathBuf;
use thiserror::Error;
use tokio::task::JoinError;

use crate::custom_resolvers::JavaScriptPackageManager;

#[derive(Error, Debug)]
pub enum ServerError {
    /// returned if the current directory path cannot be read
    #[error("could not create path '{0}' for the embedded server files")]
    CreateDir(PathBuf),

    /// returned if any of the embedded worker files cannot be written to disk
    #[error("could not write an embedded server file: {0}")]
    WriteFile(String),

    /// returned if the version of the existing worker files cannot be read
    #[error("could not read the previously extracted embedded file versions")]
    ReadVersion,

    /// returned if a connection to the sqlite database could not be made
    #[error("could not connect to the sqlite database: {0}")]
    ConnectToDatabase(SqlxError),

    /// returned if an sqlite database file cannot be created
    #[error("could not create an sqlite database file: {0}")]
    CreateDatabase(SqlxError),

    /// returned if an sqlite query returns an error
    #[error("could not query the sqlite database: {0}")]
    QueryDatabase(SqlxError),

    /// returned if sqlx returns an unknown error
    #[error("encountered an unknown sqlite error: {0}")]
    UnknownSqliteError(SqlxError),

    /// returned if the sqlite bridge cannot be started
    #[error("the bridge api encountered an error: {0}")]
    BridgeApi(#[from] HyperError),

    /// returned if the miniflare command returns an error
    #[error("miniflare encountered an error: {0}")]
    MiniflareCommandError(IoError),

    /// returned if the miniflare command exits unsuccessfully
    #[error("miniflare encountered an error\ncause:\n{0}")]
    MiniflareError(String),

    /// returned if the schema parser command returns an error
    #[error(transparent)]
    SchemaParserError(IoError),

    /// returned if reading the parser result fails
    #[error(transparent)]
    SchemaParserResultRead(IoError),

    /// returned if the schema parser result is invalid JSON
    #[error("schema parser result is malformed JSON:\n{0}")]
    SchemaParserResultJson(serde_json::Error),

    /// returned if writing the schema registry fails
    #[error(transparent)]
    SchemaRegistryWrite(IoError),

    /// returned if `tempfile::NamedTempFile::new()` fails.
    #[error("could not create a temporary file: {0}")]
    CreateTemporaryFile(IoError),

    /// returned if `tempfile::NamedTempFile::new()` fails.
    #[error("could not write to a temporary file '{0}': {1}")]
    CreateNotWriteToTemporaryFile(PathBuf, IoError),

    /// returned if a write to a resolver artifact file fails
    #[error("could not create a file {0} during a resolver build: {1}")]
    CreateResolverArtifactFile(PathBuf, IoError),

    /// returned if a write to a resolver artifact file fails
    #[error("could not read the file {0}: {1}")]
    ReadFile(PathBuf, IoError),

    /// returned if the schema parser command exits unsuccessfully
    #[error("could not extract the resolver wrapper worker contents")]
    ExtractResolverWrapperWorkerContents(String),

    /// returned if the schema parser command exits unsuccessfully
    #[error("could not parse grafbase/schema.graphql\n{0}")]
    ParseSchema(String),

    /// returned if the typescript config parser command exits unsuccessfully
    #[error("could not load grafbase/grafbase.config.ts\nCaused by: {0}")]
    LoadTsConfig(String),

    #[error("could not find a resolver referenced in the schema under the path {0}.{{js,ts}}")]
    ResolverDoesNotExist(PathBuf),

    /// returned if any of the npm commands ran during resolver build exits unsuccessfully
    #[error("{0} encountered an error: {1}")]
    ResolverPackageManagerCommandError(JavaScriptPackageManager, IoError),

    /// returned if any of the npm commands ran during resolver build exits unsuccessfully
    #[error("{0} failed with output:\n{1}")]
    ResolverPackageManagerError(JavaScriptPackageManager, String),

    /// returned if any of the npm commands ran during resolver build exits unsuccessfully
    #[error("resolver {0} failed to build:\n{1}")]
    ResolverBuild(String, String),

    /// returned if the user project path is not valid utf-8
    #[error("non utf-8 path used for project")]
    ProjectPath,

    /// returned if the user cache path is not valid utf-8
    #[error("$HOME/.grafbase is a non utf8 path")]
    CachePath,

    /// returned if the `.grafbase` directory cannot be created
    #[error("could not create a project cache directory")]
    CreateCacheDir,

    /// returned if the `.grafbase/database` directory cannot be created
    #[error("could not create a project database directory\nCaused by: {0}")]
    CreateDatabaseDir(IoError),

    /// returned if the `.grafbase/database` directory cannot be read
    #[error("could not read the project database directory\nCaused by: {0}")]
    ReadDatabaseDir(IoError),

    /// returned if an available port cannot be found for the bridge server
    #[error("could not find an available port for the bridge server")]
    AvailablePort,

    /// returned if a spawned task panics
    #[error(transparent)]
    SpawnedTaskPanic(#[from] JoinError),

    /// returned if node is not in the user $PATH
    #[error("Node.js does not seem to be installed")]
    NodeInPath,

    /// returned if the installed version of node is unsupported
    #[error("Node.js version {0} is unsupported")]
    OutdatedNode(String, String),

    /// returned if the installed version of node could not be retreived
    #[error("Could not retrive the installed version of Node.js")]
    CheckNodeVersion,

    /// returned if a file watcher could not be initialized or was stopped due to an error
    #[error("A file watcher encountered an error\nCaused by: {0}")]
    FileWatcher(#[from] NotifyError),

    #[error("Could not create a lock for the wrangler installation: {0}")]
    Lock(#[from] fslock::Error),
}

impl From<SqlxError> for ServerError {
    fn from(error: SqlxError) -> Self {
        match error {
            SqlxError::RowNotFound
            | SqlxError::TypeNotFound { .. }
            | SqlxError::ColumnNotFound(_)
            | SqlxError::ColumnDecode { .. }
            | SqlxError::ColumnIndexOutOfBounds { .. }
            | SqlxError::Io(_)
            | SqlxError::Decode(_)
            | SqlxError::Database(_) => Self::QueryDatabase(error),

            SqlxError::Configuration(_)
            | SqlxError::Tls(_)
            | SqlxError::PoolTimedOut
            | SqlxError::Protocol(_)
            | SqlxError::PoolClosed
            | SqlxError::WorkerCrashed => Self::ConnectToDatabase(error),

            SqlxError::Migrate(_) => Self::CreateDatabase(error),

            _ => Self::UnknownSqliteError(error),
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "error": self.to_string(),
        }));

        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}
