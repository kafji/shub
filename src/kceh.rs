//! Reverse of [Heck] crate.
//!
//! Only support ASCII characters.
//!
//! [Heck]: https://github.com/withoutboats/heck

#[derive(PartialEq, Copy, Clone, Debug)]
enum Input {
    /// snake_case
    SnakeCase,
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum Output {
    /// Capitalize the first letter.
    Statement,
}

fn transform(s: impl Into<String>, input: Input, output: Output) -> String {
    let s = s.into();
    let chars = s.chars().into_iter();
    let chars = match input {
        Input::SnakeCase => match output {
            Output::Statement => chars.enumerate().map(|(i, c)| {
                let c = if c == '_' { ' ' } else { c };
                let c = if i == 0 { c.to_ascii_uppercase() } else { c };
                c
            }),
        },
    };
    chars.collect()
}

/// Transform `snake_case` to `Statement`.
pub fn snake_case_to_statement(s: impl Into<String>) -> String {
    transform(s, Input::SnakeCase, Output::Statement)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_snake_case_to_statement() {
        let input = "hello_world";
        let output = snake_case_to_statement(input);
        assert_eq!("Hello world", output);
    }
}
