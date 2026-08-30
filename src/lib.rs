//! Very simple command line argument parser.
//!
//! Most argument parsers are declarative: you tell them what to parse,
//! and they do it.
//!
//! This one just provides you with a stream of options and values and lets you
//! figure out the rest.
//!
//! ## Example
//! ```no_run
//! use lexopt::{Parser, Arg};
//!
//! use Arg::*;
//!
//! type Error = Box<dyn core::error::Error>;
//!
//! struct Args {
//!     thing: String,
//!     number: u32,
//!     shout: bool,
//! }
//!
//! fn parse_args() -> Result<Args, Error> {
//!     let mut thing = None;
//!     let mut number = 1;
//!     let mut shout = false;
//!     let mut parser = Parser::new(std::env::args());
//!     while let Some(arg) = parser.next()? {
//!         match arg {
//!             Short('n') | Long("number") => {
//!                 number = parser.value()?.parse()?;
//!             }
//!             Long("shout") => {
//!                 shout = true;
//!             }
//!             Value(val) if thing.is_none() => {
//!                 thing = Some(val);
//!             }
//!             Long("help") => {
//!                 println!("Usage: hello [-n|--number=NUM] [--shout] THING");
//!                 std::process::exit(0);
//!             }
//!             _ => return Err(arg.unexpected().into()),
//!         }
//!     }
//!
//!     Ok(Args {
//!         thing: thing.ok_or("missing argument THING")?,
//!         number,
//!         shout,
//!     })
//! }
//!
//! fn main() -> Result<(), Error> {
//!     let args = parse_args()?;
//!     let mut message = format!("Hello {}", args.thing);
//!     if args.shout {
//!         message = message.to_uppercase();
//!     }
//!     for _ in 0..args.number {
//!         println!("{}", message);
//!     }
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]

/// Parser for command line arguments.
#[derive(Debug)]
pub struct Parser<I = std::env::Args> {
    args: I,
    state: State,
    /// The last option we emitted.
    last_option: LastOption,
    /// The name of the command (argv\[0\]).
    bin_name: Option<String>,
    short_equals: bool,
}

#[derive(Debug)]
enum State {
    /// Nothing interesting is going on.
    None,
    /// We have a value left over from `--option=value`.
    PendingValue(String),
    /// We're in the middle of `-abc`.
    Shorts(Vec<u8>, usize),
    /// We saw `--` and know no more options are coming.
    FinishedOpts,
}

/// We use this to keep track of the last emitted option, for error messages when
/// an expected value is not found.
///
/// We also use this as storage for long options so we can hand out `&str`.
#[derive(Debug)]
enum LastOption {
    None,
    Short(char),
    Long(String),
}

/// A command line argument found by [`Parser`], either an option or a positional argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg<'a> {
    /// A short option, e.g. `Short('q')` for `-q`.
    Short(char),
    /// A long option, e.g. `Long("verbose")` for `--verbose`. (The dashes are not included.)
    Long(&'a str),
    /// A standalone positional argument.
    Value(String),
}

impl<I> Parser<I>
where
    I: Iterator,
    I::Item: Into<String>,
{
    /// # Example
    /// ```
    /// lexopt::Parser::new(std::env::args());
    /// ```
    pub fn new<U>(args: U) -> Self
    where
        U: IntoIterator<IntoIter = I>,
    {
        let mut args = args.into_iter();
        let bin_name = args.next();
        Parser {
            args,
            state: State::None,
            last_option: LastOption::None,
            bin_name: bin_name.map(Into::into),
            short_equals: true,
        }
    }

    /// Accept just arguments except for program name (argv\[0\]).
    /// # Example
    /// ```
    /// let mut args = std::env::args();
    /// let _program_name = args.next();
    /// lexopt::Parser::from_args(args);
    /// ```
    pub fn from_args<U>(args: U) -> Self
    where
        U: IntoIterator<IntoIter = I>,
    {
        let args = args.into_iter();
        Parser {
            args,
            state: State::None,
            last_option: LastOption::None,
            bin_name: None,
            short_equals: true,
        }
    }

    /// Get the next option or positional argument.
    ///
    /// A return value of `Ok(None)` means the command line has been exhausted.
    ///
    /// # Errors
    ///
    /// [`Error::UnexpectedValue`] is returned if the last option had a
    /// value that hasn't been consumed (by calling [`value()`](Self::value)),
    /// as in `--option=value` or `-o=value`.
    ///
    /// It's possible to continue parsing after an error (but this is rarely useful).
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<Arg<'_>>, Error> {
        match self.state {
            State::PendingValue(ref mut value) => {
                // Last time we got `--long=value`, and `value` hasn't been used.
                let value = core::mem::take(value);
                self.state = State::None;
                return Err(Error::UnexpectedValue {
                    option: self
                        .format_last_option()
                        .expect("Should only have pending value after long option"),
                    value,
                });
            }
            State::Shorts(ref arg, ref mut pos) => {
                // We're somewhere inside a `-abc` chain. Because we're in `.next()`,
                // not `.value()`, we can assume that the next character is another option.
                match first_codepoint(&arg[*pos..]) {
                    Ok(None) => {
                        self.state = State::None;
                    }
                    // If we find `-=[...]` we interpret it as an option.
                    // If we find `-o=...` then there's an unexpected value.
                    // (`-=` as an option exists, see https://linux.die.net/man/1/a2ps.
                    // Though if you have one you should maybe disable short_equals.)
                    // `clap` always interprets it as a short flag in this case, but
                    // that feels sloppy.
                    Ok(Some('=')) if *pos > 1 && self.short_equals => {
                        return Err(Error::UnexpectedValue {
                            option: self.format_last_option().unwrap(),
                            value: self.optional_value().unwrap(),
                        });
                    }
                    Ok(Some(ch)) => {
                        *pos += ch.len_utf8();
                        self.last_option = LastOption::Short(ch);
                        return Ok(Some(Arg::Short(ch)));
                    }
                    Err(err) => {
                        // Advancing may allow recovery.
                        // This is a little iffy, there might be more bad unicode next.
                        match err.error_len() {
                            Some(len) => *pos += len,
                            None => *pos = arg.len(),
                        }
                        self.last_option = LastOption::Short('�');
                        return Ok(Some(Arg::Short('�')));
                    }
                }
            }
            State::FinishedOpts => {
                return Ok(self.args.next().map(|s| Arg::Value(s.into())));
            }
            State::None => (),
        }

        let arg = match self.args.next() {
            Some(arg) => arg.into(),
            None => return Ok(None),
        };

        if arg == "--" {
            self.state = State::FinishedOpts;
            return self.next();
        }

        let mut arg = arg;
        if arg.starts_with("--") {
            // Long options have two forms: `--option` and `--option=value`.
            if let Some(ind) = arg.find('=') {
                // SAFETY: length of `=` is 1 so range `ind + 1..` is always valid.
                let value = unsafe { arg.get_unchecked(ind + 1..) };
                self.state = State::PendingValue(value.to_owned());
                arg.truncate(ind);
            }
            self.last_option = LastOption::Long(arg);
            let LastOption::Long(ref option) = self.last_option else {
                // SAFETY: ensured by above.
                unsafe { core::hint::unreachable_unchecked() }
            };
            // SAFETY: ensured by the outer `if`.
            Ok(Some(Arg::Long(unsafe {
                option.get_unchecked("--".len()..)
            })))
        } else if arg.len() > 1 && arg.starts_with('-') {
            self.state = State::Shorts(arg.into_bytes(), 1);
            self.next()
        } else {
            Ok(Some(Arg::Value(arg)))
        }
    }

    /// Get a value for an option.
    ///
    /// This function should normally be called right after seeing an option
    /// that expects a value, with positional arguments being collected
    /// using [`next()`][Self::next].
    ///
    /// A value is collected even if it looks like an option
    /// (i.e., starts with `-`).
    ///
    /// # Errors
    ///
    /// An [`Error::MissingValue`] is returned if the end of the command
    /// line is reached.
    pub fn value(&mut self) -> Result<String, Error> {
        if let Some(value) = self.optional_value() {
            return Ok(value);
        }

        if let Some(value) = self.args.next() {
            return Ok(value.into());
        }

        Err(Error::MissingValue {
            option: self.format_last_option(),
        })
    }

    #[inline(never)]
    fn format_last_option(&self) -> Option<String> {
        match self.last_option {
            LastOption::None => None,
            LastOption::Short(ch) => {
                Some(["-", ch.encode_utf8(&mut [0; char::MAX_LEN_UTF8])].concat())
            }
            LastOption::Long(ref option) => Some(option.clone()),
        }
    }

    /// The name of the command, as in the zeroth argument of the process.
    ///
    /// This is intended for use in messages.
    ///
    /// To get the current executable, use [`std::env::current_exe`].
    ///
    /// # Example
    /// ```
    /// let mut parser = lexopt::Parser::new(std::env::args());
    /// let bin_name = parser.bin_name().unwrap_or("myapp");
    /// println!("{}: Some message", bin_name);
    /// ```
    pub fn bin_name(&self) -> Option<&str> {
        self.bin_name.as_deref()
    }

    /// Get a value only if it's concatenated to an option, as in `-ovalue` or
    /// `--option=value` or `-o=value`, but not `-o value` or `--option value`.
    pub fn optional_value(&mut self) -> Option<String> {
        Some(self.raw_optional_value()?.0)
    }

    /// [`Parser::optional_value`], but indicate whether the value was joined
    /// with an = sign. This matters for [`Parser::values`].
    fn raw_optional_value(&mut self) -> Option<(String, bool)> {
        match core::mem::replace(&mut self.state, State::None) {
            State::PendingValue(value) => Some((value, true)),
            State::Shorts(mut arg, mut pos) => {
                if pos >= arg.len() {
                    return None;
                }
                let mut had_eq_sign = false;
                if arg[pos] == b'=' && self.short_equals {
                    // -o=value.
                    // clap actually strips out all leading '='s, but that seems silly.
                    // We allow `-xo=value`. Python's argparse doesn't strip the = in that case.
                    pos += 1;
                    had_eq_sign = true;
                }
                // Move `arg[pos..]` forward to reuse allocation.
                arg.drain(..pos);
                // SAFETY: internal implementation should ensure this.
                debug_assert!(str::from_utf8(&arg).is_ok());
                let arg = unsafe { String::from_utf8_unchecked(arg) };
                Some((arg, had_eq_sign))
            }
            State::FinishedOpts => {
                // Not really supposed to be here, but it's benign and not our fault
                self.state = State::FinishedOpts;
                None
            }
            State::None => None,
        }
    }

    /// Configure whether to parse an equals sign (`=`) for short options.
    ///
    /// If this is **true** (the default), `-o=foobar` will be interpreted as
    /// the option `-o` with the value `foobar`.
    ///
    /// If this is **false**, `-o=foobar` will be interpreted as
    /// the option `-o` with the value `=foobar`.
    ///
    /// Note that even if this is `true` the equals sign is optional. That is,
    /// `-ofoobar` and `-o foobar` are always interpreted as `-o` with the value
    /// `foobar` regardless of this setting.
    ///
    /// Most other argument parsers treat the equals sign as part of the value,
    /// but the syntax is notably accepted by [`clap`](https://docs.rs/clap/latest/clap/)
    /// and by Python's [`argparse`](https://docs.python.org/3/library/argparse.html).
    ///
    /// You may want to disable this setting if it's common for an option's value
    /// to start with an equals sign. The Unix `cut` command for example is sometimes
    /// used with `-d=`, where `"="` is the value belonging to the `-d` option. By default
    /// the empty string `""` is parsed instead.
    ///
    /// # Example
    ///
    /// You can configure this right after creating the parser:
    /// ```
    /// let mut parser = lexopt::Parser::new(std::env::args());
    /// parser.set_short_equals(false);
    /// ```
    ///
    /// You could also do it temporarily, for an individual option:
    /// ```
    /// # fn main() -> Result<(), Box<dyn core::error::Error>> {
    /// # use lexopt::{Parser, Arg};
    /// # use Arg::*;
    /// let mut parser = lexopt::Parser::from_args(["-d=", "key=value"]);
    /// let mut delimiter = None;
    /// while let Some(arg) = parser.next()? {
    ///     match arg {
    ///         Short('d') | Long("delimiter") => {
    ///            parser.set_short_equals(false);
    ///            delimiter = Some(parser.value()?);
    ///            parser.set_short_equals(true);
    ///         }
    ///         _ => (),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_short_equals(&mut self, on: bool) {
        self.short_equals = on;
    }
}

impl Arg<'_> {
    /// Convert an unexpected argument into an error.
    pub fn unexpected(self) -> Error {
        match self {
            Arg::Short(short) => Error::UnexpectedOption(
                ["-", short.encode_utf8(&mut [0; char::MAX_LEN_UTF8])].concat(),
            ),
            Arg::Long(long) => Error::UnexpectedOption(["--", long].concat()),
            Arg::Value(value) => Error::UnexpectedArgument(value),
        }
    }
}

/// An error during argument parsing.
///
/// This implements `From<String>` and `From<&str>`, for easy ad-hoc error
/// messages.
//
// This is not #[non_exhaustive] because of the MSRV. I'm hoping no more
// variants will turn out to be needed: this seems reasonable, if the scope
// of the library doesn't change. Worst case scenario it can be stuffed inside
// Error::Custom.
pub enum Error {
    /// An option argument was expected but was not found.
    MissingValue {
        /// The most recently emitted option.
        option: Option<String>,
    },

    /// An unexpected option was found.
    UnexpectedOption(String),

    /// A positional argument was found when none was expected.
    UnexpectedArgument(String),

    /// An option had a value when none was expected.
    UnexpectedValue {
        /// The option.
        option: String,
        /// The value.
        value: String,
    },

    /// A value was found that was not valid unicode.
    ///
    /// This can be returned by the methods on [`ValueExt`].
    NonUnicodeValue(String),

    /// For custom error messages in application code.
    Custom(Box<dyn core::error::Error + Send + Sync + 'static>),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use crate::Error::*;
        match self {
            MissingValue { option: None } => write!(f, "missing argument"),
            MissingValue {
                option: Some(option),
            } => {
                write!(f, "missing argument for option '{}'", option)
            }
            UnexpectedOption(option) => write!(f, "invalid option '{}'", option),
            UnexpectedArgument(value) => write!(f, "unexpected argument {:?}", value),
            UnexpectedValue { option, value } => {
                write!(
                    f,
                    "unexpected argument for option '{}': {:?}",
                    option, value
                )
            }
            NonUnicodeValue(value) => write!(f, "argument is invalid unicode: {:?}", value),
            Custom(err) => write!(f, "{}", err),
        }
    }
}

// This is printed when returning an error from main(), so defer to Display
impl core::fmt::Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Error::Custom(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::Custom(msg.into())
    }
}

impl<'a> From<&'a str> for Error {
    fn from(msg: &'a str) -> Self {
        Error::Custom(msg.into())
    }
}

///
/// Take the first codepoint from a UTF-8 bytestring.
///
/// The rest of the bytestring does not have to be valid unicode.
fn first_codepoint(bytes: &[u8]) -> Result<Option<char>, core::str::Utf8Error> {
    // We only need the first 4 bytes
    let bytes = bytes.get(..4).unwrap_or(bytes);
    let text = match str::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) if err.valid_up_to() > 0 => str::from_utf8(&bytes[..err.valid_up_to()]).unwrap(),
        Err(err) => return Err(err),
    };
    Ok(text.chars().next())
}

#[cfg(test)]
mod tests;
