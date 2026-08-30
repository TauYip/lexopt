use core::error::Error as StdError;

use super::{Arg, Parser};

use Arg::*;

type Error = Box<dyn StdError>;

#[test]
fn test_basic() -> Result<(), Error> {
    let mut p = Parser::from_args(["-n", "10", "foo", "-", "--", "baz", "-qux"]);
    assert_eq!(p.next()?.unwrap(), Short('n'));
    assert_eq!(p.value()?.parse::<i32>()?, 10);
    assert_eq!(p.next()?.unwrap(), Value("foo".into()));
    assert_eq!(p.next()?.unwrap(), Value("-".into()));
    assert_eq!(p.next()?.unwrap(), Value("baz".into()));
    assert_eq!(p.next()?.unwrap(), Value("-qux".into()));
    assert_eq!(p.next()?, None);
    assert_eq!(p.next()?, None);
    assert_eq!(p.next()?, None);
    Ok(())
}

#[test]
fn test_combined() -> Result<(), Error> {
    let mut p = Parser::from_args(["-abc", "-fvalue", "-xfvalue"]);
    assert_eq!(p.next()?.unwrap(), Short('a'));
    assert_eq!(p.next()?.unwrap(), Short('b'));
    assert_eq!(p.next()?.unwrap(), Short('c'));
    assert_eq!(p.next()?.unwrap(), Short('f'));
    assert_eq!(p.value()?, "value");
    assert_eq!(p.next()?.unwrap(), Short('x'));
    assert_eq!(p.next()?.unwrap(), Short('f'));
    assert_eq!(p.value()?, "value");
    assert_eq!(p.next()?, None);
    Ok(())
}

#[test]
fn test_long() -> Result<(), Error> {
    let mut p = Parser::from_args(["--foo", "--bar=qux", "--foobar=qux=baz"]);
    assert_eq!(p.next()?.unwrap(), Long("foo"));
    assert_eq!(p.next()?.unwrap(), Long("bar"));
    assert_eq!(p.value()?, "qux");
    assert_eq!(p.next()?.unwrap(), Long("foobar"));
    assert_eq!(
        p.next().unwrap_err().to_string(),
        r#"unexpected argument for option '--foobar': "qux=baz""#
    );
    assert_eq!(p.next()?, None);
    Ok(())
}

#[test]
fn test_dash_args() -> Result<(), Error> {
    // "--" should indicate the end of the options
    let mut p = Parser::from_args(["-x", "--", "-y"]);
    assert_eq!(p.next()?.unwrap(), Short('x'));
    assert_eq!(p.next()?.unwrap(), Value("-y".into()));
    assert_eq!(p.next()?, None);

    // ...unless it's an argument of an option
    let mut p = Parser::from_args(["-x", "--", "-y"]);
    assert_eq!(p.next()?.unwrap(), Short('x'));
    assert_eq!(p.value()?, "--");
    assert_eq!(p.next()?.unwrap(), Short('y'));
    assert_eq!(p.next()?, None);

    // "-" is a valid value that should not be treated as an option
    let mut p = Parser::from_args(["-x", "-", "-y"]);
    assert_eq!(p.next()?.unwrap(), Short('x'));
    assert_eq!(p.next()?.unwrap(), Value("-".into()));
    assert_eq!(p.next()?.unwrap(), Short('y'));
    assert_eq!(p.next()?, None);

    // '-' is a silly and hard to use short option, but other parsers treat
    // it like an option in this position
    let mut p = Parser::from_args(["-x-y"]);
    assert_eq!(p.next()?.unwrap(), Short('x'));
    assert_eq!(p.next()?.unwrap(), Short('-'));
    assert_eq!(p.next()?.unwrap(), Short('y'));
    assert_eq!(p.next()?, None);

    Ok(())
}

#[test]
fn test_missing_value() -> Result<(), Error> {
    let mut p = Parser::from_args(["-o"]);
    assert_eq!(p.next()?.unwrap(), Short('o'));
    assert_eq!(
        p.value().unwrap_err().to_string(),
        "missing argument for option '-o'",
    );

    let mut q = Parser::from_args(["--out"]);
    assert_eq!(q.next()?.unwrap(), Long("out"));
    assert_eq!(
        q.value().unwrap_err().to_string(),
        "missing argument for option '--out'",
    );

    let mut r = Parser::from_args([""; 0]);
    assert_eq!(r.value().unwrap_err().to_string(), "missing argument");

    Ok(())
}

#[test]
fn test_weird_args() -> Result<(), Error> {
    let mut p = Parser::from_args([
        "", "--=", "--=3", "-", "-x", "--", "-", "-x", "--", "", "-", "-x",
    ]);
    assert_eq!(p.next()?.unwrap(), Value(String::from("")));

    // These are weird and questionable, but this seems to be the standard
    // interpretation
    // GNU getopt_long and argparse complain that it could be an abbreviation
    // of every single long option
    // clap complains that "--" is not expected, which matches its treatment
    // of unknown long options
    assert_eq!(p.next()?.unwrap(), Long(""));
    assert_eq!(p.value()?, String::from(""));
    assert_eq!(p.next()?.unwrap(), Long(""));
    assert_eq!(p.value()?, String::from("3"));

    assert_eq!(p.next()?.unwrap(), Value(String::from("-")));
    assert_eq!(p.next()?.unwrap(), Short('x'));
    assert_eq!(p.value()?, String::from("--"));
    assert_eq!(p.next()?.unwrap(), Value(String::from("-")));
    assert_eq!(p.next()?.unwrap(), Short('x'));
    assert_eq!(p.next()?.unwrap(), Value(String::from("")));
    assert_eq!(p.next()?.unwrap(), Value(String::from("-")));
    assert_eq!(p.next()?.unwrap(), Value(String::from("-x")));
    assert_eq!(p.next()?, None);

    let mut r = Parser::from_args([""; 0]);
    assert_eq!(r.next()?, None);

    Ok(())
}

#[test]
fn test_unicode() -> Result<(), Error> {
    let mut p = Parser::from_args(["-aµ", "--µ=10", "µ", "--foo=µ"]);
    assert_eq!(p.next()?.unwrap(), Short('a'));
    assert_eq!(p.next()?.unwrap(), Short('µ'));
    assert_eq!(p.next()?.unwrap(), Long("µ"));
    assert_eq!(p.value()?, "10");
    assert_eq!(p.next()?.unwrap(), Value("µ".into()));
    assert_eq!(p.next()?.unwrap(), Long("foo"));
    assert_eq!(p.value()?, "µ");
    Ok(())
}

#[cfg(any(unix, windows, all(target_os = "wasi", target_env = "p1")))]
#[test]
fn test_mixed_invalid() -> Result<(), Error> {
    let mut p = Parser::from_args(["--foo=@@@"]);
    assert_eq!(p.next()?.unwrap(), Long("foo"));
    assert_eq!(p.value()?, "@@@");

    let mut q = Parser::from_args(["-💣@@@"]);
    assert_eq!(q.next()?.unwrap(), Short('💣'));
    assert_eq!(q.value()?, "@@@");

    let mut r = Parser::from_args(["-f@@@"]);
    assert_eq!(r.next()?.unwrap(), Short('f'));
    assert_eq!(r.next()?.unwrap(), Short('@'));
    assert_eq!(r.next()?.unwrap(), Short('@'));
    assert_eq!(r.next()?.unwrap(), Short('@'));
    assert_eq!(r.next()?, None);

    let mut s = Parser::from_args(["--foo=bar=@@@"]);
    assert_eq!(s.next()?.unwrap(), Long("foo"));
    assert_eq!(s.value()?, "bar=@@@");

    Ok(())
}

#[cfg(any(unix, windows, all(target_os = "wasi", target_env = "p1")))]
#[test]
fn test_separate_invalid() -> Result<(), Error> {
    let mut p = Parser::from_args(["--foo", "@@@"]);
    assert_eq!(p.next()?.unwrap(), Long("foo"));
    assert_eq!(p.value()?, "@@@");
    Ok(())
}

#[cfg(any(unix, windows, all(target_os = "wasi", target_env = "p1")))]
#[test]
fn test_invalid_long_option() -> Result<(), Error> {
    let mut p = Parser::from_args(["--@=10"]);
    assert_eq!(p.next()?.unwrap(), Long("@"));
    assert_eq!(p.value().unwrap(), "10");
    assert_eq!(p.next()?, None);

    let mut q = Parser::from_args(["--@"]);
    assert_eq!(q.next()?.unwrap(), Long("@"));
    assert_eq!(q.next()?, None);

    Ok(())
}

#[test]
fn short_opt_equals_sign() -> Result<(), Error> {
    let mut p = Parser::from_args(["-a=b"]);
    assert_eq!(p.next()?.unwrap(), Short('a'));
    assert_eq!(p.value()?, "b");
    assert_eq!(p.next()?, None);

    let mut p = Parser::from_args(["-a=b"]);
    assert_eq!(p.next()?.unwrap(), Short('a'));
    assert_eq!(
        p.next().unwrap_err().to_string(),
        r#"unexpected argument for option '-a': "b""#
    );
    assert_eq!(p.next()?, None);

    let mut p = Parser::from_args(["-a="]);
    assert_eq!(p.next()?.unwrap(), Short('a'));
    assert_eq!(p.value()?, "");
    assert_eq!(p.next()?, None);

    let mut p = Parser::from_args(["-a="]);
    assert_eq!(p.next()?.unwrap(), Short('a'));
    assert_eq!(
        p.next().unwrap_err().to_string(),
        r#"unexpected argument for option '-a': """#
    );
    assert_eq!(p.next()?, None);

    let mut p = Parser::from_args(["-="]);
    assert_eq!(p.next()?.unwrap(), Short('='));
    assert_eq!(p.next()?, None);

    let mut p = Parser::from_args(["-=a"]);
    assert_eq!(p.next()?.unwrap(), Short('='));
    assert_eq!(p.value()?, "a");

    Ok(())
}

#[cfg(any(unix, windows, all(target_os = "wasi", target_env = "p1")))]
#[test]
fn short_opt_equals_sign_invalid() -> Result<(), Error> {
    let mut p = Parser::from_args(["-a=@"]);
    assert_eq!(p.next()?.unwrap(), Short('a'));
    assert_eq!(p.value()?, "@");
    assert_eq!(p.next()?, None);

    let mut p = Parser::from_args(["-=@"]);
    assert_eq!(p.next()?.unwrap(), Short('='));
    assert_eq!(p.value()?, "@");

    Ok(())
}

// #[test]
// fn multi_values() -> Result<(), Error> {
//     for &case in &["-a b c d", "-ab c d", "-a b c d --", "--a b c d"] {
//         let mut p = Parser::from_args(case.split_whitespace());
//         p.next()?.unwrap();
//         let mut iter = p.values()?;
//         let values: Vec<_> = iter.by_ref().collect();
//         assert_eq!(values, &["b", "c", "d"]);
//         assert!(iter.next().is_none());
//         assert!(p.next()?.is_none());
//     }

//     for &case in &["-a=b c", "--a=b c"] {
//         let mut p = Parser::from_args(case.split_whitespace());
//         p.next()?.unwrap();
//         let mut iter = p.values()?;
//         let values: Vec<_> = iter.by_ref().collect();
//         assert_eq!(values, &["b"]);
//         assert!(iter.next().is_none());
//         assert_eq!(p.next()?.unwrap(), Value("c".into()));
//         assert!(p.next()?.is_none());
//     }

//     for &case in &["-a", "--a", "-a -b", "-a -- b", "-a --"] {
//         let mut p = Parser::from_args(case.split_whitespace());
//         p.next()?.unwrap();
//         assert!(p.values().is_err());
//         assert!(p.next().is_ok());
//         assert!(p.next().unwrap().is_none());
//     }

//     for &case in &["-a=", "--a="] {
//         let mut p = parse(case);
//         p.next()?.unwrap();
//         let mut iter = p.values()?;
//         let values: Vec<_> = iter.by_ref().collect();
//         assert_eq!(values, &[""]);
//         assert!(iter.next().is_none());
//         assert!(p.next()?.is_none());
//     }

//     // Test that .values() does not eagerly consume the first value
//     for &case in &["-a=b", "--a=b", "-a b"] {
//         let mut p = parse(case);
//         p.next()?.unwrap();
//         assert!(p.values().is_ok());
//         assert_eq!(p.value()?, "b");
//     }

//     {
//         let mut p = parse("-ab");
//         p.next()?.unwrap();
//         assert!(p.values().is_ok());
//         assert_eq!(p.next()?.unwrap(), Short('b'));
//     }

//     Ok(())
// }

#[test]
fn short_opt_equals_sign_disabled() -> Result<(), Error> {
    let mut p = Parser::from_args(["-d=", "-d=value", "-dvalue", "-d"]);
    p.set_short_equals(false);

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.value()?, "=");

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.value()?, "=value");

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.value()?, "value");

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(
        p.value().unwrap_err().to_string(),
        "missing argument for option '-d'"
    );

    assert_eq!(p.next()?, None);

    let mut p = Parser::from_args(["-d=", "-d=value", "-dvalue", "-d"]);
    p.set_short_equals(false);

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.optional_value().unwrap(), "=");

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.optional_value().unwrap(), "=value");

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.optional_value().unwrap(), "value");

    assert_eq!(p.next()?.unwrap(), Short('d'));

    assert_eq!(p.optional_value(), None);
    assert_eq!(p.next()?, None);

    let mut p = Parser::from_args(["-d=", "-d=v", "-dv", "-d", "-="]);
    p.set_short_equals(false);

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.next()?.unwrap(), Short('='));

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.next()?.unwrap(), Short('='));
    assert_eq!(p.next()?.unwrap(), Short('v'));

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.next()?.unwrap(), Short('v'));

    assert_eq!(p.next()?.unwrap(), Short('d'));

    assert_eq!(p.next()?.unwrap(), Short('='));

    assert_eq!(p.next()?, None);

    let mut p = Parser::from_args(["-d", "1", "2", "-d1", "2", "-d=1", "2", "-d="]);
    p.set_short_equals(false);

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.value()?, "1");
    assert_eq!(p.value()?, "2");

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.value()?, "1");
    assert_eq!(p.value()?, "2");

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.value()?, "=1");
    assert_eq!(p.value()?, "2");

    assert_eq!(p.next()?.unwrap(), Short('d'));
    assert_eq!(p.value()?, "=");

    assert_eq!(p.next()?, None);

    Ok(())
}

/// It's possible to disable this setting for a single method call.
/// This might break if we parse the equals sign any earlier than needed.
#[test]
fn short_opt_equals_sign_temporarily_disabled() -> Result<(), Error> {
    let mut p = Parser::from_args(["-o=", "-o=", "-o=", "-o=", "-o=", "-o="]);
    assert_eq!(p.next()?.unwrap(), Short('o'));
    p.set_short_equals(false);
    assert_eq!(p.next()?.unwrap(), Short('='));
    p.set_short_equals(true);

    assert_eq!(p.next()?.unwrap(), Short('o'));
    assert_eq!(
        p.next().unwrap_err().to_string(),
        r#"unexpected argument for option '-o': """#,
    );

    assert_eq!(p.next()?.unwrap(), Short('o'));
    p.set_short_equals(false);
    assert_eq!(p.value()?, "=");
    p.set_short_equals(true);

    assert_eq!(p.next()?.unwrap(), Short('o'));
    assert_eq!(p.value()?, "");

    assert_eq!(p.next()?.unwrap(), Short('o'));
    p.set_short_equals(false);
    assert_eq!(p.optional_value().unwrap(), "=");
    p.set_short_equals(true);

    assert_eq!(p.next()?.unwrap(), Short('o'));
    assert_eq!(p.value()?, "");

    assert_eq!(p.next()?, None);

    Ok(())
}

#[test]
fn bin_name() {
    assert_eq!(Parser::new(["foo", "bar", "baz"]).bin_name(), Some("foo"));
    assert_eq!(Parser::from_args(["foo", "bar", "baz"]).bin_name(), None);
    assert_eq!(Parser::new([""; 0]).bin_name(), None);
    assert_eq!(Parser::new([""]).bin_name(), Some(""));
}

#[test]
fn test_errors() {
    assert_eq!(
        Arg::Short('o').unexpected().to_string(),
        "invalid option '-o'",
    );
    assert_eq!(
        Arg::Long("opt").unexpected().to_string(),
        "invalid option '--opt'",
    );
    assert_eq!(
        Arg::Value("foo".into()).unexpected().to_string(),
        r#"unexpected argument "foo""#,
    );
    assert!(Arg::Short('o').unexpected().source().is_none());
    assert_eq!(
        format!("{:?}", Arg::Short('o').unexpected()),
        "invalid option '-o'",
    );
}
