use super::*;

pub(super) fn merge_input_object_definitions(
    ctx: &mut Context<'_>,
    first: &DefinitionWalker<'_>,
    definitions: &[DefinitionWalker<'_>],
) {
    let description = definitions.iter().find_map(|def| def.description());

    let composed_directives = collect_composed_directives(definitions.iter().map(|def| def.directives()), ctx);

    ctx.insert_input_object(first.name(), description, composed_directives);

    // We want to take the intersection of the field sets.
    let intersection: HashSet<StringId> = first
        .fields()
        .map(|field| field.name().id)
        .filter(|field_name| definitions[1..].iter().all(|def| def.find_field(*field_name).is_some()))
        .collect();

    let mut all_fields: Vec<_> = definitions
        .iter()
        .flat_map(|def| def.fields())
        .map(|field| (field.name().id, field))
        .collect();

    all_fields.sort_by_key(|(name, _)| *name);

    let mut start = 0;

    while start < all_fields.len() {
        let field_name = all_fields[start].0;
        let end = all_fields[start..].partition_point(|(name, _)| *name == field_name) + start;
        let fields = &all_fields[start..end];

        start = end;

        // Check that no required field was excluded.
        if !intersection.contains(&field_name) {
            if let Some((_, required_field)) = fields.iter().find(|(_, field)| field.r#type().is_required()) {
                ctx.diagnostics.push_fatal(format!(
                    "The {input_type_name}.{field_name} field is not defined in all subgraphs, but it is required in {bad_subgraph}",
                    input_type_name = first.name().as_str(),
                    field_name = required_field.name().as_str(),
                    bad_subgraph = required_field.parent_definition().subgraph().name().as_str(),
                ));
            }
            continue;
        }

        let directive_containers = fields.iter().map(|(_, field)| field.directives());
        let composed_directives = collect_composed_directives(directive_containers, ctx);

        let description = fields
            .iter()
            .find_map(|(_, field)| field.description())
            .map(|description| ctx.insert_string(description.id));

        let Some(field_type) = fields::compose_input_field_types(fields.iter().map(|(_, field)| *field), ctx) else {
            continue;
        };

        ctx.insert_field(ir::FieldIr {
            parent_name: first.name().id,
            field_name,
            field_type,
            arguments: Vec::new(),
            resolvable_in: None,
            provides: Vec::new(),
            requires: Vec::new(),
            overrides: Vec::new(),
            composed_directives,
            description,
        });
    }
}
