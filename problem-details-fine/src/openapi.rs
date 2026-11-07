use std::{any::TypeId, collections::BTreeMap};

use http::StatusCode;
use utoipa::{
    PartialSchema,
    openapi::{
        AllOfBuilder, Content, ObjectBuilder, OneOfBuilder, RefOr, ResponseBuilder, Schema,
        SchemaFormat, Type,
        response::Response,
        schema::{AdditionalProperties, ArrayItems, SchemaType},
    },
};

use crate::{
    MEDIA_TYPE, ProblemSet, ProblemType,
    identity::TypeIdentity,
    problem::{Extensions, FixedValue, TagField, tag_collision},
};

/// One problem type in a [`ProblemSet`], for the Openapi responses.
#[derive(Debug, Clone, Copy)]
pub struct ProblemEntry {
    /// The identity of the problem type. The response assembly uses it to remove duplicates.
    pub id: TypeId,
    /// The path of the definition, `module::path::Ident`, for error messages.
    pub definition: &'static str,
    /// The type URI, after the rewrite.
    pub type_uri: &'static str,
    /// The status code of the problem type.
    pub status: StatusCode,
    /// Returns the schema of the complete document.
    pub schema: fn() -> RefOr<Schema>,
}

/// Returns the [`ProblemEntry`] of a problem type.
pub(crate) fn entry_of<T: ProblemType + PartialSchema>() -> ProblemEntry {
    ProblemEntry {
        id: TypeId::of::<T>(),
        definition: T::__ID,
        type_uri: &T::__resolved().0.type_uri,
        status: T::STATUS,
        schema: T::schema,
    }
}

/// Returns the Openapi responses of a [`ProblemSet`].
///
/// - There is one `application/problem+json` response for each status code, in ascending order.
/// - If two or more problem types have the same status, the schema is a `oneOf` of their schemas.
/// - A problem type that occurs more than once in the set occurs once in the responses.
///
/// # Panics
///
/// Panics if two different problem types in the set have the same type URI after the rewrite. `#[problem_set]`
/// finds the same problem in the definitions when it compiles. Only a rewrite rule can cause it here.
pub fn problem_responses<S: ProblemSet>() -> BTreeMap<String, RefOr<Response>> {
    let mut entries = Vec::new();
    S::__entries(&mut entries);
    assemble(entries)
}

fn assemble(entries: Vec<ProblemEntry>) -> BTreeMap<String, RefOr<Response>> {
    let mut seen: Vec<ProblemEntry> = Vec::new();
    let mut by_status: BTreeMap<StatusCode, Vec<RefOr<Schema>>> = BTreeMap::new();
    for entry in entries {
        if seen.iter().any(|seen| seen.id == entry.id) {
            continue;
        }
        if let Some(other) = seen.iter().find(|seen| seen.type_uri == entry.type_uri) {
            panic!(
                "the problem types `{}` and `{}` both present the type URI `{}`; a type URI must identify one \
                 problem type. Check the rewrite configuration.",
                other.definition, entry.definition, entry.type_uri
            );
        }
        seen.push(entry);
        by_status
            .entry(entry.status)
            .or_default()
            .push((entry.schema)());
    }

    by_status
        .into_iter()
        .map(|(status, mut schemas)| {
            let schema = if schemas.len() == 1 {
                schemas.pop().expect("one schema")
            } else {
                Schema::OneOf(
                    schemas
                        .into_iter()
                        .fold(OneOfBuilder::new(), OneOfBuilder::item)
                        .build(),
                )
                .into()
            };
            let response = ResponseBuilder::new()
                .description(status.canonical_reason().unwrap_or("Problem"))
                .content(MEDIA_TYPE, Content::new(Some(schema)))
                .build();
            (status.as_str().to_owned(), response.into())
        })
        .collect()
}

/// Returns the schema of the complete document of `T`: the schema of the standard members, and the `payload`
/// schema of the extension members.
///
/// `definitions` contains the component schemas that `payload` refers to, from `ToSchema::schemas`. This function
/// replaces each reference with its schema. Only a recursive reference stays a `$ref`.
pub fn problem_schema<T: ProblemType>(
    payload: Option<RefOr<Schema>>,
    definitions: Vec<(String, RefOr<Schema>)>,
) -> RefOr<Schema> {
    let resolved = &T::__resolved().0;
    let base = base_schema::<T>(resolved);

    let payload = payload.map(|mut payload| {
        let definitions: BTreeMap<_, _> = definitions.into_iter().collect();
        inline_refs(&mut payload, &definitions, &mut Vec::new());
        payload
    });

    if resolved.tag_member().is_some() {
        let collides = match T::__TAG_FIELD {
            TagField::No => false,
            TagField::Yes => true,
            TagField::Unknown => payload.as_ref().is_some_and(has_tag_property),
        };
        assert!(!collides, "{}", tag_collision(Some(T::__ID)));
    }

    match (T::__EXTENSIONS, payload) {
        (Extensions::None, _) | (_, None) => base.into(),
        (Extensions::Wrap(key), Some(payload)) => base.property(key, payload).required(key).into(),
        (Extensions::Flatten, Some(RefOr::T(Schema::Object(object))))
            if object.schema_type == SchemaType::Type(Type::Object) =>
        {
            let mut merged = base.build();
            merged.properties.extend(object.properties);
            for required in object.required {
                if !merged.required.contains(&required) {
                    merged.required.push(required);
                }
            }
            if merged.description.is_none() {
                merged.description = object.description;
            }
            Schema::Object(merged).into()
        }
        (Extensions::Flatten, Some(payload)) => {
            Schema::AllOf(AllOfBuilder::new().item(base).item(payload).build()).into()
        }
    }
}

fn base_schema<T: ProblemType>(resolved: &TypeIdentity<'_>) -> ObjectBuilder {
    let omit = T::__OMIT;
    let string = || ObjectBuilder::new().schema_type(SchemaType::Type(Type::String));
    let detail_description =
        "A human-readable explanation specific to this occurrence of the problem.";

    let builder = ObjectBuilder::new()
        .schema_type(SchemaType::Type(Type::Object))
        .title(Some(T::SCHEMA_TITLE))
        .property(
            "type",
            string()
                .description(Some("A URI reference that identifies the problem type."))
                .enum_values(Some([resolved.type_uri.as_ref()])),
        )
        .required("type");

    let builder = match resolved.tag_member() {
        Some(tag_uri) => builder
            .property(
                "tag",
                string()
                    .description(Some("A tag URI that identifies the problem type."))
                    .enum_values(Some([tag_uri])),
            )
            .required("tag"),
        None => builder,
    };

    let builder = builder
        .property(
            "status",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::Integer))
                .description(Some("The HTTP status code of the problem."))
                .enum_values(Some([T::STATUS.as_u16()])),
        )
        .required("status")
        .property(
            "title",
            string()
                .description(Some("A short, human-readable summary of the problem type."))
                .examples([T::TITLE]),
        )
        .required("title");

    let builder = match T::__DETAIL_AS {
        Some(name) => {
            let builder = builder.property(name, string().description(Some(detail_description)));
            if T::__DETAIL_AS_REQUIRED {
                builder.required(name)
            } else {
                builder
            }
        }
        None if omit.detail => builder,
        None => builder.property("detail", string().description(Some(detail_description))),
    };

    let builder = if omit.instance {
        builder
    } else {
        builder.property(
            "instance",
            string()
                .format(Some(SchemaFormat::Custom("uri-reference".to_owned())))
                .description(Some(
                    "A URI reference that identifies this occurrence of the problem.",
                )),
        )
    };

    T::__FIXED.iter().fold(builder, |builder, fixed| {
        let schema_type = match fixed.value {
            FixedValue::Str(_) => Type::String,
            FixedValue::Bool(_) => Type::Boolean,
            FixedValue::I64(_) | FixedValue::U64(_) => Type::Integer,
            FixedValue::F32(_) | FixedValue::F64(_) => Type::Number,
        };
        let property = ObjectBuilder::new()
            .schema_type(SchemaType::Type(schema_type))
            .description(Some(fixed.description))
            .enum_values(Some([fixed.value.to_json()]));
        builder.property(fixed.name, property).required(fixed.name)
    })
}

/// Returns `true` if the schema, or an `allOf` item of it, has a `tag` property.
fn has_tag_property(schema: &RefOr<Schema>) -> bool {
    match schema {
        RefOr::T(Schema::Object(object)) => object.properties.contains_key("tag"),
        RefOr::T(Schema::AllOf(all_of)) => all_of.items.iter().any(has_tag_property),
        _ => false,
    }
}

const COMPONENT_PREFIX: &str = "#/components/schemas/";

/// Replaces each `$ref` to a schema in `definitions` with that schema. A recursive reference stays a `$ref`.
fn inline_refs(
    schema: &mut RefOr<Schema>,
    definitions: &BTreeMap<String, RefOr<Schema>>,
    stack: &mut Vec<String>,
) {
    match schema {
        RefOr::Ref(reference) => {
            let Some(name) = reference.ref_location.strip_prefix(COMPONENT_PREFIX) else {
                return;
            };
            let Some(definition) = definitions.get(name) else {
                return;
            };
            if stack.iter().any(|seen| seen == name) {
                return;
            }
            let name = name.to_owned();
            let mut definition = definition.clone();
            stack.push(name);
            inline_refs(&mut definition, definitions, stack);
            stack.pop();
            *schema = definition;
        }
        RefOr::T(schema) => inline_schema_refs(schema, definitions, stack),
    }
}

fn inline_schema_refs(
    schema: &mut Schema,
    definitions: &BTreeMap<String, RefOr<Schema>>,
    stack: &mut Vec<String>,
) {
    match schema {
        Schema::Object(object) => {
            for property in object.properties.values_mut() {
                inline_refs(property, definitions, stack);
            }
            if let Some(additional) = object.additional_properties.as_deref_mut()
                && let AdditionalProperties::RefOr(additional) = additional
            {
                inline_refs(additional, definitions, stack);
            }
        }
        Schema::Array(array) => {
            if let ArrayItems::RefOrSchema(items) = &mut array.items {
                inline_refs(items, definitions, stack);
            }
            for item in &mut array.prefix_items {
                inline_schema_refs(item, definitions, stack);
            }
        }
        Schema::OneOf(one_of) => {
            for item in &mut one_of.items {
                inline_refs(item, definitions, stack);
            }
        }
        Schema::AllOf(all_of) => {
            for item in &mut all_of.items {
                inline_refs(item, definitions, stack);
            }
        }
        Schema::AnyOf(any_of) => {
            for item in &mut any_of.items {
                inline_refs(item, definitions, stack);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use utoipa::openapi::Ref;

    use super::*;

    fn schema(title: &str) -> RefOr<Schema> {
        ObjectBuilder::new().title(Some(title)).into()
    }

    fn entry<T: 'static>(status: StatusCode, schema: fn() -> RefOr<Schema>) -> ProblemEntry {
        ProblemEntry {
            id: TypeId::of::<T>(),
            definition: std::any::type_name::<T>(),
            type_uri: std::any::type_name::<T>(),
            status,
            schema,
        }
    }

    fn to_value<T: serde::Serialize>(value: T) -> Value {
        serde_json::to_value(value).unwrap()
    }

    fn with_uri(mut entry: ProblemEntry, type_uri: &'static str) -> ProblemEntry {
        entry.type_uri = type_uri;
        entry
    }

    #[test]
    #[should_panic(expected = "both present the type URI `https://x.test/same`")]
    fn distinct_types_sharing_a_uri_panic() {
        struct A;
        struct B;
        assemble(vec![
            with_uri(
                entry::<A>(StatusCode::NOT_FOUND, || schema("A")),
                "https://x.test/same",
            ),
            with_uri(
                entry::<B>(StatusCode::NOT_FOUND, || schema("B")),
                "https://x.test/same",
            ),
        ]);
    }

    #[test]
    #[should_panic(expected = "both present the type URI `https://x.test/same`")]
    fn distinct_types_sharing_a_uri_across_statuses_panic() {
        struct A;
        struct B;
        assemble(vec![
            with_uri(
                entry::<A>(StatusCode::NOT_FOUND, || schema("A")),
                "https://x.test/same",
            ),
            with_uri(
                entry::<B>(StatusCode::CONFLICT, || schema("B")),
                "https://x.test/same",
            ),
        ]);
    }

    #[test]
    fn responses_group_by_status_ascending_and_dedupe() {
        struct A;
        struct B;
        struct C;
        let responses = assemble(vec![
            entry::<A>(StatusCode::NOT_FOUND, || schema("A")),
            entry::<B>(StatusCode::BAD_REQUEST, || schema("B")),
            entry::<C>(StatusCode::NOT_FOUND, || schema("C")),
            entry::<A>(StatusCode::NOT_FOUND, || schema("A")),
        ]);

        assert_eq!(responses.keys().collect::<Vec<_>>(), ["400", "404"]);
        assert_eq!(
            to_value(&responses),
            json!({
                "400": {
                    "description": "Bad Request",
                    "content": {"application/problem+json": {"schema": {"type": "object", "title": "B"}}}
                },
                "404": {
                    "description": "Not Found",
                    "content": {"application/problem+json": {"schema": {"oneOf": [{"type": "object", "title": "A"}, {"type": "object", "title": "C"}]}}}
                }
            })
        );
    }

    #[test]
    fn inlines_nested_and_stops_at_recursion() {
        let definitions: BTreeMap<String, RefOr<Schema>> = [
            (
                "Outer".to_owned(),
                ObjectBuilder::new()
                    .property("inner", Ref::from_schema_name("Inner"))
                    .into(),
            ),
            (
                "Inner".to_owned(),
                ObjectBuilder::new()
                    .property("again", Ref::from_schema_name("Inner"))
                    .into(),
            ),
        ]
        .into();
        let mut schema: RefOr<Schema> = ObjectBuilder::new()
            .property("outer", Ref::from_schema_name("Outer"))
            .property("other", Ref::from_schema_name("Unknown"))
            .into();

        inline_refs(&mut schema, &definitions, &mut Vec::new());

        assert_eq!(
            to_value(&schema)["properties"],
            json!({
                "outer": {"type": "object", "properties": {"inner": {"type": "object", "properties": {
                    "again": {"$ref": "#/components/schemas/Inner"}
                }}}},
                "other": {"$ref": "#/components/schemas/Unknown"}
            })
        );
    }
}
