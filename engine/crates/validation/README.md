# graphql-schema-validation

[![docs.rs](https://img.shields.io/docsrs/graphql-schema-validation)](https://docs.rs/graphql-schema-validation)


This crate implements GraphQL SDL schema validation according to the [2021
version of the GraphQL spec](http://spec.graphql.org/October2021/).

Scope:

- All the spec and nothing but the spec.
- Query documents are out of scope, we only validate schemas.
- The error messages should be as close as possible to the style of other
  GraphQL schema validation libraries.
