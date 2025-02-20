use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use schema::{Definition, FieldId};

use crate::{
    request::{
        BoundAnyFieldDefinitionId, BoundFieldId, BoundSelectionSetId, FlatField, FlatSelectionSet, FlatTypeCondition,
        SelectionSetType,
    },
    response::{BoundResponseKey, ResponseKey},
};

use super::{BoundFieldDefinitionWalker, BoundFieldWalker, OperationWalker};

pub type FlatSelectionSetWalker<'a, Ty = SelectionSetType> = OperationWalker<'a, Cow<'a, FlatSelectionSet<Ty>>>;
pub type FlatFieldWalker<'a> = OperationWalker<'a, Cow<'a, FlatField>>;

impl<'a, Ty: Copy> FlatSelectionSetWalker<'a, Ty> {
    pub fn any_selection_set_id(&self) -> BoundSelectionSetId {
        self.wrapped.any_selection_set_id
    }

    pub fn ty(&self) -> Ty {
        self.wrapped.ty
    }

    pub fn fields(&self) -> impl ExactSizeIterator<Item = FlatFieldWalker<'_>> + '_ {
        self.wrapped
            .fields
            .iter()
            .map(move |flat_field| self.walk(Cow::Borrowed(flat_field)))
    }

    pub fn group_by_field_id(&self) -> HashMap<FieldId, GroupForFieldId<'a>> {
        self.wrapped.fields.iter().fold(HashMap::new(), |mut map, flat_field| {
            let bound_field = self.walk(flat_field.bound_field_id);
            if let Some(field) = bound_field.definition().as_field() {
                map.entry(field.id())
                    .and_modify(|group| {
                        group.key = bound_field.bound_response_key.min(group.key);
                        group.bound_field_ids.push(bound_field.id())
                    })
                    .or_insert_with(|| GroupForFieldId {
                        key: bound_field.bound_response_key,
                        field,
                        bound_field_ids: vec![bound_field.id()],
                    });
            }
            map
        })
    }

    pub fn group_by_response_key(&self) -> HashMap<ResponseKey, GroupForResponseKey> {
        self.wrapped.fields.iter().fold(
            HashMap::<ResponseKey, GroupForResponseKey>::new(),
            |mut groups, flat_field| {
                let field = &self.operation[flat_field.bound_field_id];
                let definition = &self.operation[field.definition_id];
                let group = groups
                    .entry(definition.response_key())
                    .or_insert_with(|| GroupForResponseKey {
                        key: field.bound_response_key,
                        definition_id: field.definition_id,
                        origin_selection_set_ids: HashSet::new(),
                        bound_field_ids: vec![],
                    });
                group.key = group.key.min(field.bound_response_key);
                group.bound_field_ids.push(flat_field.bound_field_id);
                group.origin_selection_set_ids.extend(&flat_field.selection_set_path);

                groups
            },
        )
    }

    pub fn into_fields(self) -> impl Iterator<Item = FlatFieldWalker<'a>> {
        let walker = self.walk(());
        self.wrapped
            .into_owned()
            .fields
            .into_iter()
            .map(move |flat_field| walker.walk(Cow::Owned(flat_field)))
    }

    pub fn partition_fields(
        mut self,
        predicate: impl Fn(FlatFieldWalker<'_>) -> bool,
    ) -> (FlatSelectionSetWalker<'a, Ty>, FlatSelectionSetWalker<'a, Ty>) {
        let fields = match self.wrapped {
            Cow::Borrowed(selection_set) => selection_set.fields.clone(),
            Cow::Owned(ref mut selection_set) => std::mem::take(&mut selection_set.fields),
        };
        let (left, right) = fields
            .into_iter()
            .partition(|flat_field| predicate(self.walk(Cow::Borrowed(flat_field))));
        (self.with_fields(left), self.with_fields(right))
    }

    fn with_fields(&self, fields: Vec<FlatField>) -> Self {
        self.walk(Cow::Owned(FlatSelectionSet {
            ty: self.wrapped.ty,
            any_selection_set_id: self.wrapped.any_selection_set_id,
            fields,
        }))
    }

    pub fn is_empty(&self) -> bool {
        self.wrapped.fields.is_empty()
    }

    pub fn into_inner(self) -> FlatSelectionSet<Ty> {
        self.wrapped.into_owned()
    }
}

impl<'a> FlatFieldWalker<'a> {
    pub fn bound_field(&self) -> BoundFieldWalker<'a> {
        self.walk(self.wrapped.bound_field_id)
    }

    pub fn into_inner(self) -> FlatField {
        self.wrapped.into_owned()
    }
}

impl<'a> std::ops::Deref for FlatFieldWalker<'a> {
    type Target = FlatField;

    fn deref(&self) -> &Self::Target {
        &self.wrapped
    }
}

pub struct GroupForFieldId<'a> {
    pub key: BoundResponseKey,
    pub field: BoundFieldDefinitionWalker<'a>,
    pub bound_field_ids: Vec<BoundFieldId>,
}

pub struct GroupForResponseKey {
    pub key: BoundResponseKey,
    pub definition_id: BoundAnyFieldDefinitionId,
    pub origin_selection_set_ids: HashSet<BoundSelectionSetId>,
    pub bound_field_ids: Vec<BoundFieldId>,
}

impl<'a, Ty: Copy + std::fmt::Debug + Into<SelectionSetType>> std::fmt::Debug for FlatSelectionSetWalker<'a, Ty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = Into::<SelectionSetType>::into(self.ty());
        let ty_name = self.walk_with(ty, Definition::from(ty)).name();

        f.debug_struct("FlatSelectionSet")
            .field("any_selection_set_id", &self.any_selection_set_id())
            .field("ty", &ty_name)
            .field("fields", &self.fields().collect::<Vec<_>>())
            .finish()
    }
}

impl<'a> std::fmt::Debug for FlatFieldWalker<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fmt = f.debug_struct("FlatField");
        if let Some(type_condition) = self.wrapped.type_condition.as_ref() {
            fmt.field("type_condition", &self.walk_with(type_condition, ()));
        }
        fmt.field("field", &self.bound_field().definition()).finish()
    }
}

impl<'a> std::fmt::Debug for OperationWalker<'a, &FlatTypeCondition, (), ()> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.wrapped {
            FlatTypeCondition::Interface(id) => f
                .debug_tuple("Inerface")
                .field(&self.schema_walker.walk(*id).name())
                .finish(),
            FlatTypeCondition::Objects(ids) => f
                .debug_tuple("Objects")
                .field(
                    &ids.iter()
                        .map(|id| self.schema_walker.walk(*id).name())
                        .collect::<Vec<_>>(),
                )
                .finish(),
        }
    }
}
