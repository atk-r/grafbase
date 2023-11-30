use std::cmp::Ordering;

use schema::{InterfaceId, ObjectId, Schema};

use super::{SelectionSetRoot, TypeCondition};
use crate::response::BoundResponseKey;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryPath(im::Vector<QueryPathSegment>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPathSegment {
    pub type_condition: Option<FlatTypeCondition>,
    pub bound_response_key: BoundResponseKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlatTypeCondition {
    Interface(InterfaceId),
    // sorted by ObjectId
    Objects(Vec<ObjectId>),
}

impl FlatTypeCondition {
    pub fn is_possible(&self) -> bool {
        match self {
            FlatTypeCondition::Interface(_) => true,
            FlatTypeCondition::Objects(ids) => !ids.is_empty(),
        }
    }

    pub fn matches(&self, schema: &Schema, object_id: ObjectId) -> bool {
        match self {
            FlatTypeCondition::Interface(id) => schema[object_id].interfaces.contains(id),
            FlatTypeCondition::Objects(ids) => ids.binary_search(&object_id).is_ok(),
        }
    }

    pub fn flatten(schema: &Schema, root: SelectionSetRoot, type_condition_chain: Vec<TypeCondition>) -> Option<Self> {
        let mut type_condition_chain = type_condition_chain.into_iter().peekable();
        let mut candidate = match root {
            SelectionSetRoot::Object(object_id) => {
                // Checking that all type conditions apply.
                for type_condition in type_condition_chain {
                    match type_condition {
                        TypeCondition::Interface(id) if schema[object_id].interfaces.contains(&id) => (),
                        TypeCondition::Object(id) if object_id == id => (),
                        TypeCondition::Union(id) if schema[id].possible_types.contains(&object_id) => (),
                        _ => return Some(FlatTypeCondition::Objects(vec![])),
                    }
                }
                return None;
            }
            SelectionSetRoot::Interface(id) => {
                // Any type condition that just applies on the root interface is ignored.
                while let Some(next) = type_condition_chain.peek().copied() {
                    if let TypeCondition::Interface(interface_id) = next {
                        if interface_id == id {
                            type_condition_chain.next();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // If there no type conditions anymore it means it just applies to the root
                // directly.
                type_condition_chain.peek()?;
                FlatTypeCondition::Interface(id)
            }
            SelectionSetRoot::Union(union_id) => {
                // Any type condition that just applies on the root union is ignored.
                while let Some(next) = type_condition_chain.peek().copied() {
                    if let TypeCondition::Union(id) = next {
                        if union_id == id {
                            type_condition_chain.next();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let first = type_condition_chain.next()?;
                match first {
                    TypeCondition::Interface(id) => FlatTypeCondition::Objects(sorted_intersection(
                        &schema[union_id].possible_types,
                        &schema[id].possible_types,
                    )),
                    TypeCondition::Object(id) => {
                        if schema[union_id].possible_types.contains(&id) {
                            FlatTypeCondition::Objects(vec![id])
                        } else {
                            FlatTypeCondition::Objects(vec![])
                        }
                    }
                    TypeCondition::Union(id) => FlatTypeCondition::Objects(sorted_intersection(
                        &schema[union_id].possible_types,
                        &schema[id].possible_types,
                    )),
                }
            }
        };

        for type_condition in type_condition_chain {
            candidate = match type_condition {
                TypeCondition::Interface(interface_id) => match candidate {
                    FlatTypeCondition::Interface(id) => {
                        if schema[interface_id].interfaces.contains(&id) {
                            FlatTypeCondition::Interface(id)
                        } else {
                            FlatTypeCondition::Objects(sorted_intersection(
                                &schema[interface_id].possible_types,
                                &schema[id].possible_types,
                            ))
                        }
                    }
                    FlatTypeCondition::Objects(ids) => {
                        FlatTypeCondition::Objects(sorted_intersection(&ids, &schema[interface_id].possible_types))
                    }
                },
                TypeCondition::Object(object_id) => match candidate {
                    FlatTypeCondition::Interface(id) => {
                        if schema[object_id].interfaces.contains(&id) {
                            FlatTypeCondition::Objects(vec![object_id])
                        } else {
                            FlatTypeCondition::Objects(vec![])
                        }
                    }
                    FlatTypeCondition::Objects(ids) => {
                        FlatTypeCondition::Objects(sorted_intersection(&ids, &[object_id]))
                    }
                },
                TypeCondition::Union(union_id) => match candidate {
                    FlatTypeCondition::Interface(id) => FlatTypeCondition::Objects(sorted_intersection(
                        &schema[union_id].possible_types,
                        &schema[id].possible_types,
                    )),
                    FlatTypeCondition::Objects(ids) => {
                        FlatTypeCondition::Objects(sorted_intersection(&ids, &schema[union_id].possible_types))
                    }
                },
            };
        }

        Some(candidate)
    }
}

fn sorted_intersection(left: &[ObjectId], right: &[ObjectId]) -> Vec<ObjectId> {
    let mut l = 0;
    let mut r = 0;
    let mut result = vec![];
    while l < left.len() && r < right.len() {
        match left[l].cmp(&right[r]) {
            Ordering::Less => l += 1,
            Ordering::Equal => {
                result.push(left[l]);
                l += 1;
                r += 1;
            }
            Ordering::Greater => r += 1,
        }
    }
    result
}

// impl TypeCondition {
//     pub fn resolve(self, schema: &Schema) -> ResolvedTypeCondition {
//         ResolvedTypeCondition {
//             possible_types: match self {
//                 TypeCondition::Interface(interface_id) => schema[interface_id].possible_types.clone(),
//                 TypeCondition::Object(object_id) => vec![object_id],
//                 TypeCondition::Union(union_id) => schema[union_id].possible_types.clone(),
//             },
//         }
//     }
// }
//
impl<'a> IntoIterator for &'a QueryPath {
    type Item = &'a QueryPathSegment;

    type IntoIter = <&'a im::Vector<QueryPathSegment> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl QueryPath {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn child(&self, segment: QueryPathSegment) -> Self {
        let mut child = self.clone();
        child.0.push_back(segment);
        child
    }
}
