# Widgets example

An `axum` server with Openapi documentation. It shows how a service uses `problem-details-fine`:

- Extractors reject a request with problem types from the `embedded` module. For example, the JSON body extractor
  returns `ParseRequestBodyProblem`, `MissingMimeProblem` or `UnsupportedMimeProblem`.
- Each operation has a problem set. Its variants hold the problems that the handler returns, and `documents(...)`
  lists the problems of the extractors.
- [`openapi.json`](openapi.json) is the generated Openapi document. Each operation has one response for each
  status. If two or more problem types have the same status, the response schema is a `oneOf`.

[`api.rs`](api.rs) has the server: the extractors, the problem sets, the handlers and the Openapi document.
[`main.rs`](main.rs) has the commands below.

## Commands

Start the server on `127.0.0.1:3000`. The server prints each request and the status of its response. For a
problem, it also prints the `type`, the `kind` and the `detail`:

```sh
cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types
```

The server accepts two bearer tokens. `reader-token` has the scope `widgets:read`. `writer-token` also has the
scope `widgets:write`.

```sh
curl -i -H 'Authorization: Bearer writer-token' -H 'Content-Type: application/json' \
    -d '{"name": "sprocket"}' http://127.0.0.1:3000/widgets
curl -i -H 'Authorization: Bearer reader-token' http://127.0.0.1:3000/widgets/abc
```

While the server runs, send it each request in [`failed-requests.md`](failed-requests.md) and print each response.
The default address is `127.0.0.1:3000`:

```sh
cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types -- showcase [ADDRESS]
```

Write `openapi.json` and [`failed-requests.md`](failed-requests.md). The command starts the server on a free port
and runs `showcase` against it:

```sh
cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types -- generate
```

The tests of the example fail if `openapi.json` or `failed-requests.md` is out of date.

## Failed requests

[`failed-requests.md`](failed-requests.md) has the output of `showcase`. A table shows the `type` and the `kind` of
each problem details document. The `kind` member names the cause, for a problem type of the `embedded` module with
more than one cause. An empty `kind` cell means that the document has no `kind` member. The full requests and responses follow the table. Before the requests, `showcase`
creates a widget with the name `sprocket`, if it does not exist.

The server also documents five problem types that are not in `failed-requests.md`:

- `UnhandledRequestBodyErrorProblem`: the server cannot read the request body. For example, a body larger than
  2 MB, the default limit of axum, causes this problem.
- `UnhandledPathParameterErrorProblem` and `UnhandledQueryStringErrorProblem`: the extractor gets a rejection that
  comes from a fault in the server, for example a route without the `{id}` parameter.
- `InternalServerErrorProblem` (500) and `DependencyProblem` (503): the store of the service fails. The in-memory
  store of the example never fails, but each operation documents these problems, as a real service does.
