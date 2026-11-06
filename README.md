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

## Server example

[`examples/widgets`](problem-details-fine/examples/widgets) is an `axum` server with Openapi documentation. Its
extractors return problem types from the `embedded` module. Its handlers return problem types from the `embedded`
module and problem types that the example defines. Each operation has a problem set.
[`failed-requests.md`](problem-details-fine/examples/widgets/failed-requests.md) shows the response to each failed
request, and [`openapi.json`](problem-details-fine/examples/widgets/openapi.json) is its Openapi document. Start the
server on `127.0.0.1:3000`:

```sh
cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types
```

## Features of `problem-details-fine`

| Feature          | Effect                                                                        |
| ---------------- | ----------------------------------------------------------------------------- |
| `utoipa`         | Adds Openapi schemas and responses for problem types and problem sets.        |
| `axum`           | Adds `IntoResponse`. Responses use the media type `application/problem+json`. |
| `rewrite`        | Adds the `rewrite` module: global rules that change type URIs and tag URIs.   |
| `embedded-types` | Adds the `embedded` module with problem types for common HTTP API errors.     |

No feature is enabled by default. The example above needs `embedded-types`.


## Development

Run the tests for each combination of features. This needs [`cargo-hack`](https://github.com/taiki-e/cargo-hack):

```sh
cargo install cargo-hack
cargo hack --feature-powerset --skip default --workspace test
```

Update the expected compiler output of the compile-fail tests:

```sh
TRYBUILD=overwrite cargo test -p problem-details-fine --features utoipa --test integration ui::
```

Update the Openapi document and the failed requests of the server example:

```sh
cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types -- generate
```

Update the snapshots:

```sh
INSTA_UPDATE=always cargo test -p problem-details-fine --all-features
```

Build the documentation with all features. `cargo docsrs` is an alias in `.cargo/config.toml` for
`cargo doc --workspace --no-deps --all-features`:

```sh
cargo docsrs
```

Build the documentation as docs.rs does, with the required feature on each item. This needs the nightly toolchain:

```sh
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly docsrs
```

Run the benchmarks. The first command measures without the `rewrite` feature. The second command measures each rewrite
configuration:

```sh
cargo bench -p problem-details-fine --bench serialize
cargo bench -p problem-details-fine --features rewrite
```

[RFC 9457]: https://www.rfc-editor.org/rfc/rfc9457.html
