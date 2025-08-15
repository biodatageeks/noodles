use std::{error, fmt, num};

use noodles_core::Position;

/// An error when a raw VCF record position fails to parse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// The input is empty.
    Empty,
    /// The input is invalid.
    Invalid(num::ParseIntError),
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Empty => None,
            Self::Invalid(e) => Some(e),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty input"),
            Self::Invalid(_) => write!(f, "invalid input"),
        }
    }
}

pub(super) fn parse_position(s: &str) -> Result<Option<Position>, ParseError> {
    const MISSING: &str = ".";

    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    match s {
        MISSING => Ok(None),
        _ => s.parse().map(Some).map_err(ParseError::Invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_position() {
        assert_eq!(parse_position("."), Ok(None));
        assert_eq!(parse_position("1"), Ok(Some(Position::MIN)));

        assert_eq!(parse_position(""), Err(ParseError::Empty));
        assert!(matches!(
            parse_position("ndls"),
            Err(ParseError::Invalid(_))
        ));
        assert!(matches!(parse_position("-1"), Err(ParseError::Invalid(_))));
        assert!(matches!(parse_position("0"), Err(ParseError::Invalid(_))));
    }
}