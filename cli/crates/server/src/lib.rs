/*!
The server crate provides a server with the gateway worker (via miniflare)
and a bridge server connecting the worker to an sqlite db

# Example

```ignore
const PORT: u16 = 4000;
# common::environment::Environment::try_init().unwrap();

// `common::environment::Environment` must be initialized before this

let server_handle = server::start(PORT).unwrap();
```
*/

// TODO: make the prior example testable

#![forbid(unsafe_code)]

#[macro_use]
extern crate log;

mod atomics;
mod bridge;
mod codegen_server;
mod consts;
mod dump_config;
mod environment;
mod error_server;
mod event;
mod file_watcher;
mod introspect_local;
mod parser;
mod proxy;
mod servers;
mod udf_builder;

pub mod errors;
pub mod types;

pub use dump_config::dump_config;
pub use introspect_local::{introspect_local, IntrospectLocalOutput};
pub use servers::{export_embedded_files, start, ProductionServer};
