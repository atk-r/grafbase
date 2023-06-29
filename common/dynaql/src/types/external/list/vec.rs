use std::borrow::Cow;

use graph_entities::ResponseNodeId;

use crate::parser::types::Field;
use crate::resolver_utils::resolve_list_native;
use crate::{
    registry, ContextSelectionSet, InputValueError, InputValueResult, LegacyInputType,
    LegacyOutputType, Positioned, Result, ServerResult, Value,
};

impl<T: LegacyInputType> LegacyInputType for Vec<T> {
    type RawValueType = Self;

    fn type_name() -> Cow<'static, str> {
        Cow::Owned(format!("[{}]", T::qualified_type_name()))
    }

    fn qualified_type_name() -> String {
        format!("[{}]!", T::qualified_type_name())
    }

    fn create_type_info(registry: &mut registry::Registry) -> String {
        T::create_type_info(registry);
        Self::qualified_type_name()
    }

    fn parse(value: Option<Value>) -> InputValueResult<Self> {
        match value.unwrap_or_default() {
            Value::List(values) => values
                .into_iter()
                .map(|value| LegacyInputType::parse(Some(value)))
                .collect::<Result<_, _>>()
                .map_err(InputValueError::propagate),
            value => Ok(vec![
                LegacyInputType::parse(Some(value)).map_err(InputValueError::propagate)?
            ]),
        }
    }

    fn to_value(&self) -> Value {
        Value::List(self.iter().map(LegacyInputType::to_value).collect())
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl<T: LegacyOutputType> LegacyOutputType for Vec<T> {
    fn type_name() -> Cow<'static, str> {
        Cow::Owned(format!("[{}]", T::qualified_type_name()))
    }

    fn qualified_type_name() -> crate::registry::MetaFieldType {
        format!("[{}]!", T::qualified_type_name()).into()
    }

    fn create_type_info(registry: &mut registry::Registry) -> crate::registry::MetaFieldType {
        T::create_type_info(registry);
        Self::qualified_type_name()
    }

    async fn resolve(
        &self,
        ctx: &ContextSelectionSet<'_>,
        field: &Positioned<Field>,
    ) -> ServerResult<ResponseNodeId> {
        resolve_list_native(ctx, field, self, Some(self.len())).await
    }
}
