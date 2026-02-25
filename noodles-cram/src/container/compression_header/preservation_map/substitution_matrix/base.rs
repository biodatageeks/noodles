use std::io;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Base {
    A,
    C,
    G,
    T,
    N,
    R, Y, K, M, S, W, B, D, H, V, // IUPAC ambiguity codes
}

impl TryFrom<u8> for Base {
    type Error = io::Error;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        match n.to_ascii_uppercase() {
            b'A' => Ok(Self::A),
            b'C' => Ok(Self::C),
            b'G' => Ok(Self::G),
            b'T' => Ok(Self::T),
            b'N' => Ok(Self::N),
            b'R' => Ok(Self::R),
            b'Y' => Ok(Self::Y),
            b'K' => Ok(Self::K),
            b'M' => Ok(Self::M),
            b'S' => Ok(Self::S),
            b'W' => Ok(Self::W),
            b'B' => Ok(Self::B),
            b'D' => Ok(Self::D),
            b'H' => Ok(Self::H),
            b'V' => Ok(Self::V),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid substitution base",
            )),
        }
    }
}

impl From<Base> for u8 {
    fn from(base: Base) -> Self {
        match base {
            Base::A => b'A',
            Base::C => b'C',
            Base::G => b'G',
            Base::T => b'T',
            Base::N => b'N',
            Base::R => b'R',
            Base::Y => b'Y',
            Base::K => b'K',
            Base::M => b'M',
            Base::S => b'S',
            Base::W => b'W',
            Base::B => b'B',
            Base::D => b'D',
            Base::H => b'H',
            Base::V => b'V',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_u8_for_base() -> io::Result<()> {
        fn t(n: u8, expected: Base) -> io::Result<()> {
            assert_eq!(Base::try_from(n)?, expected);
            assert_eq!(Base::try_from(n.to_ascii_lowercase())?, expected);
            Ok(())
        }

        t(b'A', Base::A)?;
        t(b'C', Base::C)?;
        t(b'G', Base::G)?;
        t(b'T', Base::T)?;
        t(b'N', Base::N)?;
        t(b'R', Base::R)?;
        t(b'Y', Base::Y)?;
        t(b'K', Base::K)?;
        t(b'M', Base::M)?;
        t(b'S', Base::S)?;
        t(b'W', Base::W)?;
        t(b'B', Base::B)?;
        t(b'D', Base::D)?;
        t(b'H', Base::H)?;
        t(b'V', Base::V)?;

        assert!(matches!(
            Base::try_from(b'U'),
            Err(e) if e.kind() == io::ErrorKind::InvalidData
        ));

        Ok(())
    }

    #[test]
    fn test_from_base_for_u8() {
        assert_eq!(u8::from(Base::A), b'A');
        assert_eq!(u8::from(Base::C), b'C');
        assert_eq!(u8::from(Base::G), b'G');
        assert_eq!(u8::from(Base::T), b'T');
        assert_eq!(u8::from(Base::N), b'N');
        assert_eq!(u8::from(Base::R), b'R');
        assert_eq!(u8::from(Base::Y), b'Y');
        assert_eq!(u8::from(Base::K), b'K');
        assert_eq!(u8::from(Base::M), b'M');
        assert_eq!(u8::from(Base::S), b'S');
        assert_eq!(u8::from(Base::W), b'W');
        assert_eq!(u8::from(Base::B), b'B');
        assert_eq!(u8::from(Base::D), b'D');
        assert_eq!(u8::from(Base::H), b'H');
        assert_eq!(u8::from(Base::V), b'V');
    }
}
