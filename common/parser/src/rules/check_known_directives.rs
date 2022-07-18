//! ### What it does
//!
//! Checks that schema, type and field definitions don't contain unknown directives.
//!
//! ### Why?
//!
//! Unknown directives are ignored. The user should know they have no effect.

// Based on dynaql's visitor.

use dynaql::Positioned;
use dynaql_parser::types::ConstDirective;
use dynaql_parser::types::DirectiveLocation;

use super::visitor::VisitorContext;

#[derive(Default)]
pub struct CheckAllDirectivesAreKnown {
    location_stack: Vec<DirectiveLocation>,
}

fn pretty_location_context(location: DirectiveLocation) -> &'static str {
    match location {
        DirectiveLocation::FieldDefinition => "field",
        DirectiveLocation::Schema => "schema",
        DirectiveLocation::Object => "type",
        _ => unreachable!("unexpected, other variants are never pushed onto the stack"),
    }
}

impl<'a> super::visitor::Visitor<'a> for CheckAllDirectivesAreKnown {
    fn enter_directive(&mut self, ctx: &mut VisitorContext<'a>, directive: &'a Positioned<ConstDirective>) {
        if let Some(current_location) = self.location_stack.last().copied() {
            let name_node = &directive.node.name.node;
            if let Some(schema_directive) = ctx.directives.get(directive.node.name.node.as_str()) {
                if !schema_directive
                    .node
                    .locations
                    .iter()
                    .any(|location| location.node == current_location)
                {
                    ctx.report_error(
                        vec![directive.pos],
                        format!(
                            "Directive `{name_node}` may not be used in {context} context",
                            context = pretty_location_context(current_location)
                        ),
                    );
                }
            } else {
                ctx.report_error(
                    vec![directive.pos],
                    format!(
                        "Unknown directive `{name_node}` in {context} context",
                        context = pretty_location_context(current_location)
                    ),
                );
            }
        }
    }

    fn enter_field(
        &mut self,
        _ctx: &mut super::visitor::VisitorContext<'a>,
        _field: &'a dynaql::Positioned<dynaql_parser::types::FieldDefinition>,
        _parent_type: &'a dynaql::Positioned<dynaql_parser::types::TypeDefinition>,
    ) {
        self.location_stack.push(DirectiveLocation::FieldDefinition);
    }

    fn exit_field(
        &mut self,
        _ctx: &mut super::visitor::VisitorContext<'a>,
        _field: &'a dynaql::Positioned<dynaql_parser::types::FieldDefinition>,
        _parent_type: &'a dynaql::Positioned<dynaql_parser::types::TypeDefinition>,
    ) {
        self.location_stack.pop();
    }

    fn enter_schema(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _doc: &'a dynaql::Positioned<dynaql_parser::types::SchemaDefinition>,
    ) {
        self.location_stack.push(DirectiveLocation::Schema);
    }

    fn exit_schema(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _doc: &'a dynaql::Positioned<dynaql_parser::types::SchemaDefinition>,
    ) {
        self.location_stack.pop();
    }

    fn enter_type_definition(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _type_definition: &'a dynaql::Positioned<dynaql_parser::types::TypeDefinition>,
    ) {
        self.location_stack.push(DirectiveLocation::Object);
    }

    fn exit_type_definition(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _type_definition: &'a dynaql::Positioned<dynaql_parser::types::TypeDefinition>,
    ) {
        self.location_stack.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::CheckAllDirectivesAreKnown;
    use crate::rules::{
        self,
        visitor::{visit, Visitor, VisitorContext},
    };
    use dynaql_parser::parse_schema;
    use serde_json as _;

    #[test]
    fn should_error_on_unknown_directive_in_field_position() {
        let schema = r#"
            type Product {
                id: ID!
                name: String! @break
            }
            "#;

        let mut rules = rules::visitor::VisitorNil
            .with(rules::model_directive::ModelDirective)
            .with(rules::auth_directive::AuthDirective)
            .with(rules::relations::relations_rules())
            .with(CheckAllDirectivesAreKnown::default());
        let schema = format!("{}\n{}", rules.directives(), schema);
        let schema = parse_schema(schema).expect("");
        let mut ctx = VisitorContext::new(&schema);
        visit(&mut rules, &mut ctx, &schema);

        assert_eq!(ctx.errors.len(), 1, "should have one error: {:?}", ctx.errors);
        assert_eq!(
            ctx.errors.get(0).unwrap().message,
            "Unknown directive `break` in field context",
            "should match"
        );
    }

    #[test]
    fn should_error_on_unknown_directive_in_schema_position() {
        let schema = r#"
            schema @break {
                query: Boolean
            }
            "#;

        let mut rules = rules::visitor::VisitorNil
            .with(rules::model_directive::ModelDirective)
            .with(rules::auth_directive::AuthDirective)
            .with(rules::relations::relations_rules())
            .with(CheckAllDirectivesAreKnown::default());
        let schema = format!("{}\n{}", rules.directives(), schema);
        let schema = parse_schema(schema).expect("");
        let mut ctx = VisitorContext::new(&schema);
        visit(&mut rules, &mut ctx, &schema);

        assert_eq!(ctx.errors.len(), 1, "should have one error: {:?}", ctx.errors);
        assert_eq!(
            ctx.errors.get(0).unwrap().message,
            "Unknown directive `break` in schema context",
            "should match"
        );
    }

    #[test]
    fn should_error_on_unknown_directive_in_type_position() {
        let schema = r#"
            type Product @break {
                id: ID!
                name: String!
            }
            "#;

        let mut rules = rules::visitor::VisitorNil
            .with(rules::model_directive::ModelDirective)
            .with(rules::auth_directive::AuthDirective)
            .with(rules::relations::relations_rules())
            .with(CheckAllDirectivesAreKnown::default());
        let schema = format!("{}\n{}", rules.directives(), schema);
        let schema = parse_schema(schema).expect("");
        let mut ctx = VisitorContext::new(&schema);
        visit(&mut rules, &mut ctx, &schema);

        assert_eq!(ctx.errors.len(), 1, "should have one error: {:?}", ctx.errors);
        assert_eq!(
            ctx.errors.get(0).unwrap().message,
            "Unknown directive `break` in type context",
            "should match"
        );
    }

    #[test]
    fn should_error_on_known_directive_not_allowed_in_field_position() {
        let schema = r#"
            type Product {
                id: ID!
                name: String! @model
            }
            "#;

        let mut rules = rules::visitor::VisitorNil
            .with(rules::model_directive::ModelDirective)
            .with(rules::auth_directive::AuthDirective)
            .with(rules::relations::relations_rules())
            .with(CheckAllDirectivesAreKnown::default());
        let schema = format!("{}\n{}", rules.directives(), schema);
        let schema = parse_schema(schema).expect("");
        let mut ctx = VisitorContext::new(&schema);
        visit(&mut rules, &mut ctx, &schema);

        assert_eq!(ctx.errors.len(), 1, "should have one error: {:?}", ctx.errors);
        assert_eq!(
            ctx.errors.get(0).unwrap().message,
            "Directive `model` may not be used in field context",
            "should match"
        );
    }

    #[test]
    fn should_error_on_known_directive_not_allowed_in_schema_position() {
        let schema = r#"
            schema @unique {
                query: Boolean
            }
            "#;

        let mut rules = rules::visitor::VisitorNil
            .with(rules::model_directive::ModelDirective)
            .with(rules::auth_directive::AuthDirective)
            .with(rules::relations::relations_rules())
            .with(CheckAllDirectivesAreKnown::default());
        let schema = format!("{}\n{}", rules.directives(), schema);
        let schema = parse_schema(schema).expect("");
        let mut ctx = VisitorContext::new(&schema);
        visit(&mut rules, &mut ctx, &schema);

        assert_eq!(ctx.errors.len(), 1, "should have one error: {:?}", ctx.errors);
        assert_eq!(
            ctx.errors.get(0).unwrap().message,
            "Directive `unique` may not be used in schema context",
            "should match"
        );
    }

    #[test]
    fn should_error_on_known_directive_not_allowed_in_type_position() {
        let schema = r#"
            type Product @relation {
                id: ID!
                name: String!
            }
            "#;

        let mut rules = rules::visitor::VisitorNil
            .with(rules::model_directive::ModelDirective)
            .with(rules::auth_directive::AuthDirective)
            .with(rules::relations::relations_rules())
            .with(CheckAllDirectivesAreKnown::default());
        let schema = format!("{}\n{}", rules.directives(), schema);
        let schema = parse_schema(schema).expect("");
        let mut ctx = VisitorContext::new(&schema);
        visit(&mut rules, &mut ctx, &schema);

        assert_eq!(ctx.errors.len(), 1, "should have one error: {:?}", ctx.errors);
        assert_eq!(
            ctx.errors.get(0).unwrap().message,
            "Directive `relation` may not be used in type context",
            "should match"
        );
    }
}
