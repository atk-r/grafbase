use super::{
    client::create_client,
    consts::API_URL,
    errors::ApiError,
    graphql::mutations::{FederatedGraphCompositionError, PublishPayload, SubgraphCreateArguments, SubgraphPublish},
};
use cynic::{http::ReqwestExt, MutationBuilder};

pub async fn publish(
    // The Good Code™
    account: &str,
    project: &str,
    branch: Option<&str>,
    subgraph_name: &str,
    url: &str,
    schema: &str,
) -> Result<Result<(), Vec<String>>, ApiError> {
    let client = create_client().await?;

    let operation = SubgraphPublish::build(SubgraphCreateArguments {
        input: super::graphql::mutations::PublishInput {
            account_slug: account,
            project_slug: project,
            branch,
            subgraph: subgraph_name,
            url,
            schema,
        },
    });

    let result = client.post(API_URL).run_graphql(operation).await?;

    match result.data.as_ref().and_then(|data| data.publish.as_ref()) {
        Some(PublishPayload::PublishSuccess(_)) => Ok(Ok(())),
        Some(PublishPayload::FederatedGraphCompositionError(FederatedGraphCompositionError { messages })) => {
            Ok(Err(messages.clone()))
        }
        _ => Err(ApiError::PublishError(format!("API error:\n\n{result:#?}",))),
    }
}
