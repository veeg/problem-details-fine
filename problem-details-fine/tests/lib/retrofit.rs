//! Retrofit: `detail_as` of `#[problem_type]` writes the rendered detail to an extension member instead of `detail`,
//! and `fixed(...)` adds extension members with a constant value.

use problem_details_fine::{ProblemDetails, ProblemType, problem_type};
use serde_json::json;

/// A problem type that writes its detail to `error`.
#[problem_type(
    type = "tag:fixtures.test,2025:WIDGET_CONFLICT",
    status = CONFLICT,
    title = "A widget with that id already exists",
    detail = "widget `{id}` already exists",
    detail_as = "error",
    fixed(
        /// The legacy error code. It does not change between releases.
        code: &str = "WIDGET_CONFLICT",
        /// Always `false` for errors.
        success: bool = false,
        /// A member whose name is not a Rust identifier.
        "error-kind": &str = "conflict",
    ),
)]
#[derive(Debug, Clone)]
struct WidgetConflict {
    id: String,
}

/// An enum where one variant has its own detail and the other uses the container detail.
#[problem_type(
    type = "tag:fixtures.test,2025:WIDGET_ENUM",
    status = BAD_REQUEST,
    title = "Widget",
    detail = "widget `{id}` is invalid",
    detail_as = "error",
)]
#[derive(Debug, Clone)]
#[serde(tag = "kind")]
enum WidgetEnum {
    #[problem(detail = "widget `{id}` is missing")]
    Missing {
        id: String,
    },
    Invalid {
        id: String,
    },
}

/// An enum where one variant has no detail, and there is no container detail.
#[problem_type(
    type = "tag:fixtures.test,2025:PARTIAL_DETAIL",
    status = BAD_REQUEST,
    title = "Partial",
    detail_as = "error",
)]
#[derive(Debug, Clone)]
#[serde(tag = "kind")]
enum PartialDetail {
    #[problem(detail = "widget `{id}` is missing")]
    Missing {
        id: String,
    },
    Other {
        id: String,
    },
}

fn no_detail(_: &MaybeDetail) -> Option<String> {
    None
}

/// A problem type whose detail function returns `None`.
#[problem_type(
    type = "tag:fixtures.test,2025:MAYBE_DETAIL",
    status = BAD_REQUEST,
    title = "Maybe",
    detail_with = no_detail,
    detail_as = "error",
)]
#[derive(Debug, Clone)]
struct MaybeDetail {
    id: String,
}

fn conflict() -> WidgetConflict {
    WidgetConflict { id: "w-1".into() }
}

#[test]
fn detail_as_moves_detail() {
    let value = serde_json::to_value(conflict()).unwrap();
    assert_eq!(value["error"], json!("widget `w-1` already exists"));
    assert_eq!(value.get("detail"), None);
    assert_eq!(
        conflict().detail().as_deref(),
        Some("widget `w-1` already exists")
    );
}

#[test]
fn detail_as_through_problem_details_from() {
    let details = ProblemDetails::from(conflict()).with_instance("/w/1");
    assert_eq!(details.detail(), None);
    assert_eq!(
        details.extensions()["error"],
        json!("widget `w-1` already exists")
    );
    let value = serde_json::to_value(&details).unwrap();
    assert_eq!(value["instance"], json!("/w/1"));
    assert_eq!(value["error"], json!("widget `w-1` already exists"));
    assert_eq!(value.get("detail"), None);
}

#[test]
fn detail_as_enum_variants() {
    let missing = serde_json::to_value(WidgetEnum::Missing { id: "a".into() }).unwrap();
    assert_eq!(missing["error"], json!("widget `a` is missing"));
    assert_eq!(missing.get("detail"), None);
    let invalid = serde_json::to_value(WidgetEnum::Invalid { id: "b".into() }).unwrap();
    assert_eq!(invalid["error"], json!("widget `b` is invalid"));
    assert_eq!(invalid.get("detail"), None);

    let missing = serde_json::to_value(PartialDetail::Missing { id: "c".into() }).unwrap();
    assert_eq!(missing["error"], json!("widget `c` is missing"));
    let other = serde_json::to_value(PartialDetail::Other { id: "d".into() }).unwrap();
    assert_eq!(other.get("error"), None);
}

#[test]
fn detail_as_with_detail_with_none_is_absent() {
    let value = serde_json::to_value(MaybeDetail { id: "x".into() }).unwrap();
    assert_eq!(
        value,
        json!({
            "type": "tag:fixtures.test,2025:MAYBE_DETAIL",
            "status": 400,
            "title": "Maybe",
            "id": "x",
        })
    );
}

#[cfg(feature = "utoipa")]
#[test]
fn detail_as_schema() {
    use serde_json::Value;
    use utoipa::PartialSchema;

    /// The schema, and each `allOf` item of it.
    fn parts<T: PartialSchema>() -> Vec<Value> {
        let schema = serde_json::to_value(T::schema()).unwrap();
        match schema.get("allOf") {
            Some(Value::Array(items)) => items.clone(),
            _ => vec![schema],
        }
    }
    fn has_property<T: PartialSchema>(name: &str) -> bool {
        parts::<T>()
            .iter()
            .any(|part| part["properties"].get(name).is_some())
    }
    fn is_required<T: PartialSchema>(name: &str) -> bool {
        parts::<T>().iter().any(|part| {
            part["required"]
                .as_array()
                .is_some_and(|required| required.contains(&json!(name)))
        })
    }

    assert!(has_property::<WidgetConflict>("error"));
    assert!(!has_property::<WidgetConflict>("detail"));
    assert!(is_required::<WidgetConflict>("error"));

    assert!(has_property::<MaybeDetail>("error"));
    assert!(!is_required::<MaybeDetail>("error"));

    assert!(has_property::<WidgetEnum>("error"));
    assert!(is_required::<WidgetEnum>("error"));

    assert!(has_property::<PartialDetail>("error"));
    assert!(!is_required::<PartialDetail>("error"));
}

/// A problem type without fields, with `detail_as` and `fixed`.
#[problem_type(
    type = "tag:fixtures.test,2025:UNIT_RETROFIT",
    status = BAD_REQUEST,
    title = "Unit",
    detail = "x",
    detail_as = "message",
    fixed(/** The code. */ code: &str = "UNIT"),
)]
#[derive(Debug, Clone)]
struct UnitRetrofit;

/// A problem type with `wrap`, `detail_as` and `fixed`.
#[problem_type(
    type = "tag:fixtures.test,2025:WRAPPED_RETROFIT",
    status = BAD_REQUEST,
    title = "Wrapped",
    detail = "widget `{id}`",
    detail_as = "error",
    wrap = "data",
    fixed(
        /// The code.
        code: &str = "WRAPPED",
    ),
)]
#[derive(Debug, Clone)]
struct WrappedRetrofit {
    id: String,
}

/// serde's `rename_all` does not rename a fixed member.
#[problem_type(
    type = "tag:fixtures.test,2025:RENAMED_RETROFIT",
    status = BAD_REQUEST,
    title = "Renamed",
    fixed(
        /// The code.
        error_code: &str = "X",
    ),
)]
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct RenamedRetrofit {
    widget_id: String,
}

/// Fixed members with numeric types.
#[problem_type(
    type = "tag:fixtures.test,2025:NUMERIC_RETROFIT",
    status = BAD_REQUEST,
    title = "Numeric",
    fixed(
        /// A negative integer.
        small: i8 = -3,
        /// The largest `u64`.
        big: u64 = 18446744073709551615,
        /// A `f64`.
        double: f64 = 1.25,
        /// A `f32` that is not exact in binary.
        single: f32 = 0.1,
    ),
)]
#[derive(Debug, Clone)]
struct NumericRetrofit;

#[test]
fn fixed_members_serialize_in_order() {
    assert_eq!(
        serde_json::to_string(&conflict()).unwrap(),
        r#"{"type":"tag:fixtures.test,2025:WIDGET_CONFLICT","status":409,"title":"A widget with that id already exists","error":"widget `w-1` already exists","code":"WIDGET_CONFLICT","success":false,"error-kind":"conflict","id":"w-1"}"#
    );
}

#[test]
fn fixed_members_through_problem_details_from() {
    let details = ProblemDetails::from(conflict());
    let extensions = details.extensions();
    assert_eq!(extensions["code"], json!("WIDGET_CONFLICT"));
    assert_eq!(extensions["success"], json!(false));
    assert_eq!(extensions["error-kind"], json!("conflict"));
}

#[test]
fn unit_struct_with_retrofit() {
    assert_eq!(
        serde_json::to_value(UnitRetrofit).unwrap(),
        json!({
            "type": "tag:fixtures.test,2025:UNIT_RETROFIT",
            "status": 400,
            "title": "Unit",
            "message": "x",
            "code": "UNIT",
        })
    );
    #[cfg(feature = "utoipa")]
    {
        let schema =
            serde_json::to_value(<UnitRetrofit as utoipa::PartialSchema>::schema()).unwrap();
        assert!(schema["properties"].get("message").is_some(), "{schema:#}");
        assert_eq!(
            schema["properties"]["code"]["description"],
            json!("The code.")
        );
    }
}

#[test]
fn wrap_keeps_retrofit_members_top_level() {
    let value = serde_json::to_value(WrappedRetrofit { id: "w".into() }).unwrap();
    assert_eq!(value["error"], json!("widget `w`"));
    assert_eq!(value["code"], json!("WRAPPED"));
    assert_eq!(value["data"], json!({ "id": "w" }));
}

#[test]
fn rename_all_does_not_touch_fixed() {
    let value = serde_json::to_value(RenamedRetrofit {
        widget_id: "w".into(),
    })
    .unwrap();
    assert_eq!(value["error_code"], json!("X"));
    assert_eq!(value["widgetId"], json!("w"));
    assert_eq!(value.get("errorCode"), None);
}

#[test]
fn numeric_fixed_values() {
    assert_eq!(
        serde_json::to_string(&NumericRetrofit).unwrap(),
        r#"{"type":"tag:fixtures.test,2025:NUMERIC_RETROFIT","status":400,"title":"Numeric","small":-3,"big":18446744073709551615,"double":1.25,"single":0.1}"#
    );
}

#[cfg(feature = "utoipa")]
#[test]
fn fixed_schema() {
    use utoipa::PartialSchema;

    let schema = serde_json::to_value(WidgetConflict::schema()).unwrap();
    let properties = &schema["properties"];
    assert_eq!(
        properties["code"],
        json!({
            "type": "string",
            "enum": ["WIDGET_CONFLICT"],
            "description": "The legacy error code. It does not change between releases.",
        })
    );
    assert_eq!(properties["success"]["type"], json!("boolean"));
    assert_eq!(properties["success"]["enum"], json!([false]));
    let required = schema["required"].as_array().unwrap();
    for name in ["code", "success", "error-kind"] {
        assert!(required.contains(&json!(name)), "{name} in {required:?}");
    }

    let schema = serde_json::to_value(NumericRetrofit::schema()).unwrap();
    let properties = &schema["properties"];
    assert_eq!(properties["single"]["type"], json!("number"));
    assert_eq!(properties["single"]["enum"], json!([0.1]));
    assert_eq!(properties["small"]["type"], json!("integer"));
    assert_eq!(properties["big"]["enum"], json!([18446744073709551615u64]));
    let required = schema["required"].as_array().unwrap();
    for name in ["small", "big", "double", "single"] {
        assert!(required.contains(&json!(name)), "{name} in {required:?}");
    }
}

/// A `fixed` value that arrives through a `macro_rules!` wrapper as `$v:expr`, and a parenthesized negative
/// literal. Both parse as a literal wrapped in an `Expr::Group` or `Expr::Paren`, not as a literal directly.
///
/// `#[rustfmt::skip]`: rustfmt reaches a stable format of a multi-line attribute macro call in a `macro_rules!` body
/// only after one extra run. The indentation of that stable format is unreadable.
#[rustfmt::skip]
macro_rules! retrofit_with_macro_fixed_value {
    ($v:expr) => {
        #[problem_type(
            type = "tag:fixtures.test,2025:MACRO_LITERAL_RETROFIT",
            status = BAD_REQUEST,
            title = "Macro literal",
            fixed(
                /// Passed through a `macro_rules!` `$v:expr` fragment.
                code: i32 = $v,
                /// A parenthesized negative literal.
                paren: i32 = (-1),
            ),
        )]
        #[derive(Debug, Clone)]
        struct MacroLiteralRetrofit;
    };
}

retrofit_with_macro_fixed_value!(-5);

#[test]
fn fixed_value_through_macro_rules_and_parens() {
    let value = serde_json::to_value(MacroLiteralRetrofit).unwrap();
    assert_eq!(value["code"], json!(-5));
    assert_eq!(value["paren"], json!(-1));
}
