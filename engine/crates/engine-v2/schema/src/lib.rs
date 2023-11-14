#![allow(dead_code)]

use std::{borrow::Cow, cmp::Ordering};

mod conversion;
mod ids;
pub use ids::*;

/// This does NOT need to be backwards compatible. We'll probably cache it for performance, but it is not
/// the source of truth. If the cache is stale we would just re-create this Graph from its source:
/// federated_graph::FederatedGraph.
pub struct Schema {
    data_sources: Vec<DataSource>,
    subgraphs: Vec<Subgraph>,

    pub root_operation_types: RootOperationTypes,
    // pub typename_field_id: FieldId,
    objects: Vec<Object>,
    // Sorted by object_id, name
    object_fields: Vec<ObjectField>,

    interfaces: Vec<Interface>,
    // Sorted by interface_id
    interface_fields: Vec<InterfaceField>,

    fields: Vec<Field>,

    enums: Vec<Enum>,
    unions: Vec<Union>,
    scalars: Vec<Scalar>,
    input_objects: Vec<InputObject>,
    resolvers: Vec<Resolver>,

    /// All the strings in the supergraph, deduplicated.
    strings: Vec<String>,

    /// All the field types in the supergraph, deduplicated.
    field_types: Vec<FieldType>,
}

pub struct RootOperationTypes {
    pub query: ObjectId,
    pub mutation: Option<ObjectId>,
    pub subscription: Option<ObjectId>,
}

impl std::fmt::Debug for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Schema>()).finish_non_exhaustive()
    }
}

pub enum DataSource {
    Subgraph(SubgraphId),
}

pub struct Subgraph {
    pub name: StringId,
    pub url: StringId,
}

pub struct Object {
    pub name: StringId,

    pub implements_interfaces: Vec<InterfaceId>,

    /// All _resolvable_ keys.
    pub resolvable_keys: Vec<Key>,

    /// All directives that made it through composition. Notably includes `@tag`.
    pub composed_directives: Vec<Directive>,
}

#[derive(PartialOrd, Ord, PartialEq, Eq)]
pub struct ObjectField {
    pub object_id: ObjectId,
    pub field_id: FieldId,
}

pub struct Key {
    /// The subgraph that can resolve the entity with these fields.
    pub subgraph_id: SubgraphId,

    /// Corresponds to the fields in an `@key` directive.
    pub fields: FieldSet,
}

#[derive(Default, Clone)]
pub struct FieldSet {
    // sorted by field id
    items: Vec<FieldSetItem>,
}

impl FromIterator<FieldSetItem> for FieldSet {
    fn from_iter<T: IntoIterator<Item = FieldSetItem>>(iter: T) -> Self {
        let mut items = iter.into_iter().collect::<Vec<_>>();
        items.sort_unstable_by_key(|selection| selection.field);
        Self { items }
    }
}

impl IntoIterator for FieldSet {
    type Item = FieldSetItem;

    type IntoIter = <Vec<FieldSetItem> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a FieldSet {
    type Item = &'a FieldSetItem;

    type IntoIter = <&'a Vec<FieldSetItem> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl FieldSet {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &FieldSetItem> + '_ {
        self.items.iter()
    }

    pub fn selection(&self, field: FieldId) -> Option<&FieldSetItem> {
        let index = self
            .items
            .binary_search_by_key(&field, |selection| selection.field)
            .ok()?;
        Some(&self.items[index])
    }

    pub fn merge(left_set: &FieldSet, right_set: &FieldSet) -> FieldSet {
        let mut items = vec![];
        let mut l = 0;
        let mut r = 0;
        while l < left_set.items.len() && r < right_set.items.len() {
            let left = &left_set.items[l];
            let right = &right_set.items[r];
            match left.field.cmp(&right.field) {
                Ordering::Less => {
                    items.push(left.clone());
                    l += 1;
                }
                Ordering::Equal => {
                    items.push(right.clone());
                    r += 1;
                }
                Ordering::Greater => {
                    items.push(FieldSetItem {
                        field: left.field,
                        subselection: Self::merge(&left.subselection, &right.subselection),
                    });
                    l += 1;
                    r += 1;
                }
            }
        }
        FieldSet { items }
    }
}

#[derive(Clone)]
pub struct FieldSetItem {
    pub field: FieldId,
    pub subselection: FieldSet,
}

pub struct Field {
    pub name: StringId,
    pub field_type_id: FieldTypeId,
    pub resolvers: Vec<FieldResolver>,

    /// Special case when only going through this field children are accessible.
    provides: Vec<FieldProvides>,

    pub arguments: Vec<FieldArgument>,

    /// All directives that made it through composition. Notably includes `@tag`.
    pub composed_directives: Vec<Directive>,
}

impl Field {
    pub fn provides(&self, data_source_id: DataSourceId) -> Option<&FieldSet> {
        self.provides
            .iter()
            .find(|provides| provides.data_source_id == data_source_id)
            .map(|provides| &provides.fields)
    }
}

pub struct FieldResolver {
    pub resolver_id: ResolverId,
    pub requires: FieldSet,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Resolver {
    Subgraph(SubgraphResolver),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubgraphResolver {
    pub subgraph_id: SubgraphId,
}

impl Resolver {
    pub fn data_source_id(&self) -> DataSourceId {
        match self {
            Resolver::Subgraph(resolver) => DataSourceId::from(resolver.subgraph_id),
        }
    }

    pub fn requires(&self) -> Cow<'_, FieldSet> {
        Cow::Owned(FieldSet::default())
    }
}

pub struct FieldArgument {
    pub name: StringId,
    pub type_id: FieldTypeId,
}

pub struct Directive {
    pub name: StringId,
    pub arguments: Vec<(StringId, Value)>,
}

pub enum Value {
    String(StringId),
    Int(i64),
    Float(StringId),
    Boolean(bool),
    EnumValue(StringId),
    Object(Vec<(StringId, Value)>),
    List(Vec<Value>),
}

pub enum Definition {
    Scalar(ScalarId),
    Object(ObjectId),
    Interface(InterfaceId),
    Union(UnionId),
    Enum(EnumId),
    InputObject(InputObjectId),
}

pub struct FieldType {
    pub kind: Definition,
    pub wrapping: Wrapping,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wrapping {
    /// Is the innermost type required?
    ///
    /// Examples:
    ///
    /// - `String` => false
    /// - `String!` => true
    /// - `[String!]` => true
    /// - `[String]!` => false
    pub inner_is_required: bool,

    /// Outermost to innermost.
    pub list_wrapping: Vec<ListWrapping>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListWrapping {
    RequiredList,
    NullableList,
}

/// Represents an `@provides` directive on a field in a subgraph.
pub struct FieldProvides {
    pub data_source_id: DataSourceId,
    pub fields: FieldSet,
}

pub struct Interface {
    pub name: StringId,

    /// All directives that made it through composition. Notably includes `@tag`.
    pub composed_directives: Vec<Directive>,
}

pub struct InterfaceField {
    pub interface_id: InterfaceId,
    pub field_id: FieldId,
}

pub struct Enum {
    pub name: StringId,
    pub values: Vec<EnumValue>,

    /// All directives that made it through composition. Notably includes `@tag`.
    pub composed_directives: Vec<Directive>,
}

pub struct EnumValue {
    pub value: StringId,

    /// All directives that made it through composition. Notably includes `@tag`.
    pub composed_directives: Vec<Directive>,
}

pub struct Union {
    pub name: StringId,
    pub members: Vec<ObjectId>,

    /// All directives that made it through composition. Notably includes `@tag`.
    pub composed_directives: Vec<Directive>,
}

pub struct Scalar {
    pub name: StringId,

    /// All directives that made it through composition. Notably includes `@tag`.
    pub composed_directives: Vec<Directive>,
}

pub struct InputObject {
    pub name: StringId,
    pub fields: Vec<InputObjectField>,

    /// All directives that made it through composition. Notably includes `@tag`.
    pub composed_directives: Vec<Directive>,
}

pub struct InputObjectField {
    pub name: StringId,
    pub field_type_id: FieldTypeId,
}

impl Schema {
    pub fn object_fields(&self, target: ObjectId) -> impl Iterator<Item = FieldId> + '_ {
        let start = self
            .object_fields
            .partition_point(|object_field| object_field.object_id < target);
        self.object_fields[start..].iter().map_while(move |object_field| {
            if object_field.object_id == target {
                Some(object_field.field_id)
            } else {
                None
            }
        })
    }

    pub fn interface_fields(&self, target: InterfaceId) -> impl Iterator<Item = FieldId> + '_ {
        let start = self
            .interface_fields
            .partition_point(|interface_field| interface_field.interface_id < target);
        self.interface_fields[start..].iter().map_while(move |interface_field| {
            if interface_field.interface_id == target {
                Some(interface_field.field_id)
            } else {
                None
            }
        })
    }
}
