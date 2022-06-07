use dynomite::{Attribute, AttributeValue, DynamoDbExt};
use futures_util::TryStreamExt;
use quick_error::quick_error;
use rusoto_dynamodb::QueryInput;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::dataloader::{DataLoader, Loader, LruCache};
use crate::{DynamoDBContext, DynamoDBRequestedIndex};

// TODO: Should ensure Rosoto Errors impl clone
quick_error! {
    #[derive(Debug, Clone)]
    pub enum QueryLoaderError {
        UnknownError {
            display("An internal error happened")
        }
        QueryError {
            display("An internal error happened while fetching a list of entities")
        }
    }
}

pub struct QueryLoader {
    ctx: Arc<DynamoDBContext>,
    index: DynamoDBRequestedIndex,
}

#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub struct QueryKey {
    pk: String,
    edges: Vec<String>,
}

impl QueryKey {
    pub fn new(pk: String, mut edges: Vec<String>) -> Self {
        Self {
            pk,
            edges: {
                edges.sort();
                edges
            },
        }
    }
}

#[async_trait::async_trait]
impl Loader<QueryKey> for QueryLoader {
    type Value = HashMap<String, Vec<HashMap<String, AttributeValue>>>;
    type Error = QueryLoaderError;

    async fn load(&self, keys: &[QueryKey]) -> Result<HashMap<QueryKey, Self::Value>, Self::Error> {
        log::info!(self.ctx.trace_id, "Query Dataloader invoked {:?}", keys);
        let mut h = HashMap::new();
        let mut concurrent_f = vec![];
        for query_key in keys {
            let mut exp = dynomite::attr_map! {
                ":pk" => query_key.pk.clone(),
            };
            let edges_len = query_key.edges.len();

            let mut exp_attr = HashMap::with_capacity(2);
            exp_attr.insert("#pk".to_string(), self.index.pk());

            if edges_len > 0 {
                exp_attr.insert("#type".to_string(), "__type".to_string());
            }

            let sk_string = if edges_len > 0 {
                Some(
                    query_key
                        .edges
                        .iter()
                        .enumerate()
                        .map(|(index, q)| {
                            exp.insert(format!(":type{}", index), q.clone().into_attr());
                            format!(" begins_with(#type, :type{})", index)
                        })
                        .fold(String::new(), |acc, cur| {
                            if !acc.is_empty() {
                                format!("{} OR {}", cur, acc)
                            } else {
                                cur
                            }
                        }),
                )
            } else {
                None
            };

            let input: QueryInput = QueryInput {
                table_name: self.ctx.dynamodb_table_name.clone(),
                key_condition_expression: Some("#pk = :pk".to_string()),
                filter_expression: sk_string,
                index_name: self.index.to_index_name(),
                expression_attribute_values: Some(exp),
                expression_attribute_names: Some(exp_attr),

                ..Default::default()
            };
            let future_get = || async move {
                self.ctx
                    .dynamodb_client
                    .clone()
                    .query_pages(input)
                    .try_fold(
                        (query_key.clone(), HashMap::with_capacity(100)),
                        |(query_key, mut acc), curr| async move {
                            let partition = curr
                                .get("__sk")
                                .and_then(|x| x.s.as_ref())
                                .and_then(|x| {
                                    query_key
                                        .edges
                                        .iter()
                                        .find(|edge| x.starts_with(format!("{}#", edge).as_str()))
                                })
                                .map(std::clone::Clone::clone)
                                .unwrap_or("no_partition".to_string());

                            match acc.entry(partition) {
                                Entry::Vacant(vac) => {
                                    vac.insert(vec![curr]);
                                }
                                Entry::Occupied(mut old) => {
                                    old.get_mut().push(curr);
                                }
                            }
                            Ok((query_key, acc))
                        },
                    )
                    .await
            };
            concurrent_f.push(future_get());
        }

        let b = futures_util::future::try_join_all(concurrent_f).await.map_err(|err| {
            log::error!(self.ctx.trace_id, "Error while querying: {:?}", err);
            QueryLoaderError::QueryError
        })?;

        for (q, r) in b {
            h.insert(q, r);
        }

        log::info!(self.ctx.trace_id, "Query Dataloader executed {:?}", keys);
        Ok(h)
    }
}

pub fn get_loader_query(ctx: Arc<DynamoDBContext>, index: DynamoDBRequestedIndex) -> DataLoader<QueryLoader, LruCache> {
    DataLoader::with_cache(
        QueryLoader { ctx, index },
        wasm_bindgen_futures::spawn_local,
        LruCache::new(256),
    )
    .max_batch_size(10)
    .delay(Duration::from_millis(2))
}
