# problem-details-fine

A Rust library for [RFC 9457] problem details in HTTP APIs.

Define each problem type once, as a Rust struct or enum, with the `problem_type` attribute macro. A value of the type
is one occurrence of the problem. The value serializes as the complete problem details document, and the type gives
the Openapi schema of that document.

Define the set of problem types that an operation can return with the `problem_set` attribute macro on an enum. A
handler returns the enum directly. The active variant serializes as its complete problem details document. The enum
documents one Openapi response for each status code of its problem types.

## Example

```rust
use problem_details_fine::{problem_set, problem_type};

/// Define a problem type describing a widget with the same id already exists.
#[problem_type(
    type = "https://example.com/problems/widget-conflict",
    status = CONFLICT,
    title = "A widget with that id already exists",
    detail = "widget `{id}` already exists",
)]
#[derive(Debug, Clone)]
struct WidgetConflict {
    id: String,
}

/// Combine the set of problem types that is returned by `POST /widgets`.
#[problem_set(documents(problem_details_fine::embedded::MissingRequestBodyProblem))]
#[derive(Debug)]
enum CreateWidgetError {
    Conflict(WidgetConflict),
}

fn create_widget(id: &str) -> Result<(), CreateWidgetError> {
    if id == "w-1" {
        Err(WidgetConflict { id: id.into() })?;
    }
    Ok(())
}
```

The `?` operator converts `WidgetConflict` into `CreateWidgetError`. The error from `create_widget("w-1")` serializes as
this document:

```json
{
  "type": "https://example.com/problems/widget-conflict",
  "status": 409,
  "title": "A widget with that id already exists",
  "detail": "widget `w-1` already exists",
  "id": "w-1"
}
```

The `documents(...)` argument adds `MissingRequestBodyProblem` to the Openapi documentation of the handler. The enum
cannot hold `MissingRequestBodyProblem`.

## Retrofit

An API can already have an error format of its own, for example:

```json
{ "error": "widget `w-1` already exists", "code": "WIDGET_CONFLICT", "success": false }
```

With `detail_as`, `detail_duplicate` and `fixed(...)`, the API can send problem details documents that keep each member
its clients read. `detail_as` writes the rendered detail to the extension member that you name, instead of `detail`.
`detail_duplicate` writes it to a second member and keeps `detail`. `fixed(...)` adds extension members with a constant
value, such as a legacy `code`:

```rust
#[problem_type(
    type = "https://example.com/problems/widget-conflict",
    status = CONFLICT,
    title = "A widget with that id already exists",
    detail = "widget `{id}` already exists",
    detail_as = "error",
    fixed(
        /// The legacy error code. It does not change between releases.
        code: &str = "WIDGET_CONFLICT",
        /// Always `false` for errors.
        success: bool = false,
    ),
)]
struct WidgetConflict {
    id: String,
}
```

`WidgetConflict { id: "w-1".into() }` serializes as this document:

```json
{
  "type": "https://example.com/problems/widget-conflict",
  "status": 409,
  "title": "A widget with that id already exists",
  "error": "widget `w-1` already exists",
  "code": "WIDGET_CONFLICT",
  "success": false,
  "id": "w-1"
}
```
