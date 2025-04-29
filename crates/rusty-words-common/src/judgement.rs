// TODO: Make this more advanced
use clap::ValueEnum;
use lazy_regex::regex_replace_all;
use std::borrow::Borrow;

#[derive(ValueEnum, Debug, Clone)]
pub enum TryMethod {
    /// Literally type the definition
    Write,
    /// Multiple choice (choose 1, 2, 3, 4)
    Mpc,
}

pub fn check_word<'a, S: Borrow<str>>(method: &TryMethod, input: &'a str, check: &'a [S]) -> bool {
    !check.is_empty()
        && (check_word_(method, input, check) || check_word_(method, input, &[check.join(", ")]))
}

fn check_word_<'a, S: Borrow<str>>(method: &TryMethod, input: &'a str, check: &'a [S]) -> bool {
    check.iter().any(|x| match method {
        TryMethod::Write => {
            let input = input.trim();
            let x = &x.borrow().trim();
            let y = regex_replace_all!(r#"\(.*\)"#, &x, "");
            let y = y.trim();
            let z = x.replace(['(', ')', ' '], "");
            let z = z.trim();
            input.eq_ignore_ascii_case(x)
                || input.eq_ignore_ascii_case(y)
                || input.eq_ignore_ascii_case(z)
        }
        TryMethod::Mpc => input == x.borrow(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        assert!(!check_word::<&str>(&TryMethod::Write, "", &[]));
        assert!(check_word(&TryMethod::Write, "", &[""]));
        assert!(!check_word(&TryMethod::Write, "", &["foo"]));
        assert!(!check_word(&TryMethod::Write, "", &["foo", "bar", "baz"]));

        assert!(!check_word::<&str>(&TryMethod::Mpc, "", &[]));
        assert!(check_word(&TryMethod::Mpc, "", &[""]));
        assert!(!check_word(&TryMethod::Mpc, "", &["foo"]));
        assert!(!check_word(&TryMethod::Mpc, "", &["foo", "bar", "baz"]));
    }

    #[test]
    fn test_single() {
        assert!(check_word(&TryMethod::Write, "foo", &["foo"]));
        assert!(check_word(&TryMethod::Write, "bar", &["bar"]));
        assert!(!check_word(&TryMethod::Write, "barz", &["bar"]));

        assert!(check_word(&TryMethod::Mpc, "foo", &["foo"]));
        assert!(check_word(&TryMethod::Mpc, "bar", &["bar"]));
        assert!(!check_word(&TryMethod::Mpc, "barz", &["bar"]));
    }

    #[test]
    fn test_one_of_multiple() {
        assert!(check_word(&TryMethod::Write, "foo", &["baz", "foo"]));
        assert!(check_word(&TryMethod::Write, "bar", &["baz", "bar", "baz"]));
        assert!(!check_word(
            &TryMethod::Write,
            "barz",
            &["foo", "baz", "bar"]
        ));

        assert!(check_word(&TryMethod::Mpc, "foo", &["baz", "foo"]));
        assert!(check_word(&TryMethod::Mpc, "bar", &["baz", "bar", "baz"]));
        assert!(!check_word(&TryMethod::Mpc, "barz", &["foo", "baz", "bar"]));
    }

    #[test]
    fn test_all_of_multiple() {
        assert!(check_word(&TryMethod::Write, "baz, foo", &["baz", "foo"]));
        assert!(check_word(&TryMethod::Write, "baz,foo", &["baz", "foo"]));
        assert!(check_word(
            &TryMethod::Write,
            "baz, bar, baz",
            &["baz", "bar", "baz"]
        ));

        assert!(check_word(&TryMethod::Mpc, "baz, foo", &["baz", "foo"]));
        assert!(check_word(
            &TryMethod::Mpc,
            "baz, bar, baz",
            &["baz", "bar", "baz"]
        ));
        // In multiple choice mode, only match exactly
        assert!(!check_word(
            &TryMethod::Mpc,
            "baz,bar,baz",
            &["baz", "bar", "baz"]
        ));
        assert!(!check_word(&TryMethod::Mpc, "barz", &["foo", "baz", "bar"]));
    }

    #[test]
    fn test_mixed_case() {
        assert!(check_word(&TryMethod::Write, "fOo", &["foo"]));
        assert!(check_word(&TryMethod::Write, "foo", &["FOO"]));
        assert!(check_word(&TryMethod::Write, "baR", &["bar"]));
        assert!(!check_word(&TryMethod::Write, "BARZ", &["bar"]));

        // In multiple choice mode, only match exactly
        assert!(!check_word(&TryMethod::Mpc, "Foo", &["foo"]));
        assert!(!check_word(&TryMethod::Mpc, "Bar", &["bar"]));
    }

    #[test]
    fn test_sentence() {
        assert!(check_word(
            &TryMethod::Write,
            "the quick brown fox, jumped over the lazy dog.",
            &["The quick brown fox, jumped over the lazy dog."]
        ));

        assert!(!check_word(
            &TryMethod::Mpc,
            "the quick brown fox, jumped over the lazy dog.",
            &["The quick brown fox, jumped over the lazy dog."]
        ));
    }

    #[test]
    fn test_parens() {
        assert!(check_word(
            &TryMethod::Write,
            "Such (optional)",
            &["Such (optional)"]
        ));
        assert!(!check_word(
            &TryMethod::Write,
            "Such (optional)",
            &["Such optional"]
        ));
        assert!(check_word(&TryMethod::Write, "Such", &["Such (optional)"]));

        assert!(check_word(
            &TryMethod::Mpc,
            "Such (optional)",
            &["Such (optional)"]
        ));
        assert!(!check_word(
            &TryMethod::Mpc,
            "Such (optional)",
            &["Such optional"]
        ));
        assert!(!check_word(&TryMethod::Mpc, "Such", &["Such (optional)"]));
    }

    #[test]
    fn test_trailing_spaces() {
        assert!(check_word(&TryMethod::Write, "  foo bar  ", &["foo bar"]));

        assert!(!check_word(&TryMethod::Mpc, "  foo bar  ", &["foo bar"]));
    }
}
