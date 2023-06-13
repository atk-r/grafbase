mod utils;

use backend::project::ConfigType;
use utils::consts::ENVIRONMENT_SCHEMA;
use utils::environment::Environment;

#[test]
fn environment_process() {
    let mut env = Environment::init();

    env.grafbase_init(ConfigType::GraphQL);

    env.write_schema(ENVIRONMENT_SCHEMA);

    std::env::set_var("ISSUER_URL", "https://example.com");

    env.grafbase_dev();

    let client = env.create_client();

    client.poll_endpoint(30, 300);
}

// TODO: add a test for precedence once we have a way to print variables
// (the .env variables are higher priority than process enviroment variables)
