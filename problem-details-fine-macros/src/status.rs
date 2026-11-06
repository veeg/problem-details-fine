/// The 4xx and 5xx status codes that have a named constant in `http::StatusCode`.
const NAMED: &[(u16, &str)] = &[
    (400, "BAD_REQUEST"),
    (401, "UNAUTHORIZED"),
    (402, "PAYMENT_REQUIRED"),
    (403, "FORBIDDEN"),
    (404, "NOT_FOUND"),
    (405, "METHOD_NOT_ALLOWED"),
    (406, "NOT_ACCEPTABLE"),
    (407, "PROXY_AUTHENTICATION_REQUIRED"),
    (408, "REQUEST_TIMEOUT"),
    (409, "CONFLICT"),
    (410, "GONE"),
    (411, "LENGTH_REQUIRED"),
    (412, "PRECONDITION_FAILED"),
    (413, "PAYLOAD_TOO_LARGE"),
    (414, "URI_TOO_LONG"),
    (415, "UNSUPPORTED_MEDIA_TYPE"),
    (416, "RANGE_NOT_SATISFIABLE"),
    (417, "EXPECTATION_FAILED"),
    (418, "IM_A_TEAPOT"),
    (421, "MISDIRECTED_REQUEST"),
    (422, "UNPROCESSABLE_ENTITY"),
    (423, "LOCKED"),
    (424, "FAILED_DEPENDENCY"),
    (425, "TOO_EARLY"),
    (426, "UPGRADE_REQUIRED"),
    (428, "PRECONDITION_REQUIRED"),
    (429, "TOO_MANY_REQUESTS"),
    (431, "REQUEST_HEADER_FIELDS_TOO_LARGE"),
    (451, "UNAVAILABLE_FOR_LEGAL_REASONS"),
    (500, "INTERNAL_SERVER_ERROR"),
    (501, "NOT_IMPLEMENTED"),
    (502, "BAD_GATEWAY"),
    (503, "SERVICE_UNAVAILABLE"),
    (504, "GATEWAY_TIMEOUT"),
    (505, "HTTP_VERSION_NOT_SUPPORTED"),
    (506, "VARIANT_ALSO_NEGOTIATES"),
    (507, "INSUFFICIENT_STORAGE"),
    (508, "LOOP_DETECTED"),
    (510, "NOT_EXTENDED"),
    (511, "NETWORK_AUTHENTICATION_REQUIRED"),
];

/// Parses `status = NOT_FOUND`, `status = StatusCode::NOT_FOUND` or `status = 404` into a 4xx or 5xx code.
pub(crate) fn parse(expr: &syn::Expr) -> syn::Result<u16> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(int),
            ..
        }) => {
            let code: u16 = int.base10_parse()?;
            if (400..=599).contains(&code) {
                Ok(code)
            } else {
                Err(syn::Error::new_spanned(
                    int,
                    "a problem type `status` must be a client or server error (400-599)",
                ))
            }
        }
        syn::Expr::Path(path) => {
            let ident = path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
                .unwrap_or_default();
            NAMED
                .iter()
                .find(|(_, name)| *name == ident)
                .map(|(code, _)| *code)
                .ok_or_else(|| {
                    syn::Error::new_spanned(
                        path,
                        format!(
                            "`{ident}` is not a 4xx/5xx `http::StatusCode` constant; use a name like `NOT_FOUND` or a number like `404`"
                        ),
                    )
                })
        }
        other => Err(syn::Error::new_spanned(
            other,
            "expected a `http::StatusCode` constant name like `NOT_FOUND` or a number like `404`",
        )),
    }
}
