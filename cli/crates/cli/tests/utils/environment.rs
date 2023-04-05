#![allow(dead_code)]

use super::async_client::AsyncClient;
use super::kill_with_children::kill_with_children;
use super::{cargo_bin::cargo_bin, client::Client};
use duct::{cmd, Handle};
use std::io;
use std::process::Output;
use std::sync::Arc;
use std::{env, fs, io::Write, path::PathBuf};
use tempfile::{tempdir, TempDir};

pub struct Environment {
    pub endpoint: String,
    pub directory: PathBuf,
    temp_dir: Arc<TempDir>,
    schema_path: PathBuf,
    commands: Vec<Handle>,
    port: u16,
}

const DOT_ENV_FILE: &str = ".env";

fn get_free_port() -> u16 {
    const INITIAL_PORT: u16 = 4000;

    let test_state_directory_path = std::env::temp_dir().join("grafbase/cli-tests");
    std::fs::create_dir_all(&test_state_directory_path).unwrap();
    let lock_file_path = test_state_directory_path.join("port-number.lock");
    let port_number_file_path = test_state_directory_path.join("port-number.txt");
    let mut lock_file = fslock::LockFile::open(&lock_file_path).unwrap();
    lock_file.lock().unwrap();
    let port_number = if port_number_file_path.exists() {
        std::fs::read_to_string(&port_number_file_path)
            .unwrap()
            .trim()
            .parse::<u16>()
            .unwrap()
            + 1
    } else {
        INITIAL_PORT
    };
    std::fs::write(&port_number_file_path, port_number.to_string()).unwrap();
    lock_file.unlock().unwrap();
    port_number
}

impl Environment {
    pub fn init() -> Self {
        let port = get_free_port();

        let temp_dir = Arc::new(tempdir().unwrap());
        env::set_current_dir(temp_dir.path()).unwrap();

        let schema_path = temp_dir.path().join("grafbase").join("schema.graphql");

        Self {
            directory: temp_dir.path().to_owned(),
            commands: vec![],
            endpoint: format!("http://127.0.0.1:{port}/graphql"),
            schema_path,
            temp_dir,
            port,
        }
    }

    pub fn from(other: &Environment) -> Self {
        let port = get_free_port();

        let temp_dir = other.temp_dir.clone();

        Self {
            directory: other.directory.clone(),
            commands: vec![],
            endpoint: format!("http://127.0.0.1:{port}/graphql"),
            schema_path: other.schema_path.clone(),
            temp_dir,
            port,
        }
    }

    pub fn create_client(&self) -> Client {
        Client::new(self.endpoint.clone())
    }

    pub fn create_async_client(&self) -> AsyncClient {
        AsyncClient::new(self.endpoint.clone())
    }

    pub fn write_schema(&self, schema: impl AsRef<str>) {
        fs::write(&self.schema_path, schema.as_ref()).unwrap();
    }

    pub fn write_resolver(&self, path: impl AsRef<str>, contents: impl AsRef<str>) {
        let target_path = self
            .schema_path
            .parent()
            .unwrap()
            .join("resolvers")
            .join(path.as_ref())
            .with_extension("js");
        fs::create_dir_all(target_path.parent().unwrap()).unwrap();
        fs::write(target_path, contents.as_ref()).unwrap();
    }

    pub fn grafbase_init(&self) {
        cmd!(cargo_bin("grafbase"), "init").dir(&self.directory).run().unwrap();
    }

    pub fn grafbase_init_output(&self) -> Output {
        cmd!(cargo_bin("grafbase"), "init")
            .dir(&self.directory)
            .stderr_capture()
            .unchecked()
            .run()
            .unwrap()
    }

    pub fn grafbase_init_template_output(&self, name: Option<&str>, template: &str) -> Output {
        if let Some(name) = name {
            cmd!(cargo_bin("grafbase"), "init", name, "--template", template)
        } else {
            cmd!(cargo_bin("grafbase"), "init", "--template", template)
        }
        .dir(&self.directory)
        .stderr_capture()
        .unchecked()
        .run()
        .unwrap()
    }

    pub fn grafbase_init_template(&self, name: Option<&str>, template: &str) {
        if let Some(name) = name {
            cmd!(cargo_bin("grafbase"), "init", name, "--template", template)
        } else {
            cmd!(cargo_bin("grafbase"), "init", "--template", template)
        }
        .dir(&self.directory)
        .run()
        .unwrap();
    }

    pub fn remove_grafbase_dir(&self, name: Option<&str>) {
        let directory = name.map_or_else(|| self.directory.join("grafbase"), |name| self.directory.join(name));
        fs::remove_dir_all(directory).unwrap();
    }

    pub fn grafbase_dev(&mut self) {
        let command = cmd!(
            cargo_bin("grafbase"),
            "--trace",
            "2",
            "dev",
            "--disable-watch",
            "--port",
            self.port.to_string()
        )
        .dir(&self.directory)
        .start()
        .unwrap();

        self.commands.push(command);
    }

    pub fn grafbase_dev_output(&mut self) -> io::Result<Output> {
        cmd!(
            cargo_bin("grafbase"),
            "dev",
            "--disable-watch",
            "--port",
            self.port.to_string()
        )
        .dir(&self.directory)
        .start()?
        .into_output()
    }

    pub fn set_variables<K, V>(&mut self, variables: impl IntoIterator<Item = (K, V)>)
    where
        K: std::fmt::Display,
        V: std::fmt::Display,
    {
        let env_file = variables
            .into_iter()
            .map(|(key, value)| format!(r#"{key}="{value}""#))
            .collect::<Vec<_>>()
            .join("\n");

        std::fs::write(
            self.schema_path.parent().expect("must exist").join(DOT_ENV_FILE),
            env_file,
        )
        .unwrap();
    }

    pub fn grafbase_reset(&mut self) {
        cmd!(cargo_bin("grafbase"), "reset").dir(&self.directory).run().unwrap();
    }

    pub fn grafbase_dev_watch(&mut self) {
        let command = cmd!(cargo_bin("grafbase"), "dev", "--port", self.port.to_string())
            .dir(&self.directory)
            .start()
            .unwrap();

        self.commands.push(command);
    }

    pub fn append_to_schema(&self, contents: &'static str) {
        let mut file = fs::OpenOptions::new().append(true).open(&self.schema_path).unwrap();

        file.write_all(format!("\n{contents}").as_bytes()).unwrap();

        file.sync_all().unwrap();

        drop(file);
    }

    pub fn kill_processes(&mut self) {
        self.commands.iter().for_each(|command| {
            kill_with_children(*command.pids().first().unwrap());
        });

        self.commands = vec![];
    }

    pub fn has_dot_grafbase_directory(&mut self) -> bool {
        fs::metadata(self.directory.clone().join(".grafbase")).is_ok()
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        self.kill_processes();
    }
}
