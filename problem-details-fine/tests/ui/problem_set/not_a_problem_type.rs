use problem_details_fine::problem_set;

#[derive(serde::Serialize)]
struct Plain;

#[problem_set]
enum Set {
    Plain(Plain),
}

fn main() {}
