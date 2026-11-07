use problem_details_fine::problem_type;

struct Code;

#[problem_type(
    type = "tag:x.test,2025:A",
    status = NOT_FOUND,
    title = "X",
    fixed(
        /// Doc.
        letter: char = 'a',
        /// Doc.
        owned: String = "a",
        /// Doc.
        custom: Code = 1,
    ),
)]
struct Unsupported;

#[problem_type(
    type = "tag:x.test,2025:B",
    status = NOT_FOUND,
    title = "X",
    fixed(
        /// Doc.
        code: &str = 5,
        /// Doc.
        n: u8 = -1,
        /// Doc.
        x: f64 = 1,
    ),
)]
struct Mismatch;

fn main() {}
