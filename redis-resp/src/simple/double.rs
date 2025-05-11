use crate::error::Error;

pub const PREFIX: u8 = b',';

#[derive(Debug, PartialEq, Clone)]
pub struct Double(pub f64);

impl TryFrom<&[u8]> for Double {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let data = super::data_subslice(bytes, PREFIX)?;
        let s = std::str::from_utf8(data).map_err(|_| Error::InvalidEncoding)?;
        let value = match s {
            "inf" => f64::INFINITY,
            "-inf" => f64::NEG_INFINITY,
            "nan" => f64::NAN,
            _ => s.parse().map_err(|_| Error::InvalidEncoding)?,
        };
        Ok(Double(value))
    }
}

impl From<Double> for Vec<u8> {
    fn from(d: Double) -> Self {
        let d = if d.0.is_infinite() && d.0.is_sign_positive() {
            "inf".to_string()
        } else if d.0.is_infinite() && d.0.is_sign_negative() {
            "-inf".to_string()
        } else if d.0.is_nan() {
            "nan".to_string()
        } else {
            format!("{}", d.0)
        };

        format!("{}{}\r\n", PREFIX as char, d).into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod serializa {
        use super::*;

        #[test]
        fn double_decimal() {
            let d = Double(1.23);
            let bytes: Vec<u8> = d.into();
            assert_eq!(&bytes, b",1.23\r\n");
        }

        #[test]
        fn double_entero() {
            let d = Double(10.0);
            let bytes: Vec<u8> = d.into();
            assert_eq!(&bytes, b",10\r\n");
        }

        #[test]
        fn double_negativo() {
            let d = Double(-1.23);
            let bytes: Vec<u8> = d.into();
            assert_eq!(&bytes, b",-1.23\r\n");
        }

        #[test]
        fn double_infinito_positivo() {
            let d = Double(f64::INFINITY);
            let bytes: Vec<u8> = d.into();
            assert_eq!(&bytes, b",inf\r\n");
        }

        #[test]
        fn double_infinito_negativo() {
            let d = Double(f64::NEG_INFINITY);
            let bytes: Vec<u8> = d.into();
            assert_eq!(&bytes, b",-inf\r\n");
        }

        #[test]
        fn double_nan() {
            let d = Double(f64::NAN);
            let bytes: Vec<u8> = d.into();
            assert_eq!(&bytes, b",nan\r\n");
        }

        #[test]
        fn double_con_exponente() {
            let d = Double(1.23e2);
            let bytes: Vec<u8> = d.into();
            // sugiero mandemos sin el exponente y listo pero igual lo soportamos..
            assert!(bytes == b",123\r\n");
        }
    }

    mod deserializa {
        use super::*;

        #[test]
        fn double_decimal() {
            let bytes = ",1.23\r\n".as_bytes();
            let d = Double::try_from(bytes).unwrap();
            assert_eq!(d, Double(1.23));
        }

        #[test]
        fn double_entero() {
            let bytes = ",10\r\n".as_bytes();
            let d = Double::try_from(bytes).unwrap();
            assert_eq!(d, Double(10.0));
        }

        #[test]
        fn double_negativo() {
            let bytes = ",-1.23\r\n".as_bytes();
            let d = Double::try_from(bytes).unwrap();
            assert_eq!(d, Double(-1.23));
        }

        #[test]
        fn double_con_exponente_minuscula_e() {
            let bytes = ",1.23e2\r\n".as_bytes();
            //soportamos el exponente
            let d = Double::try_from(bytes).unwrap();
            assert_eq!(d, Double(123.0));
        }

        #[test]
        fn double_con_exponente_mayuscula_e() {
            let bytes = ",1.23E2\r\n".as_bytes();
            //soportamos el exponente
            let d = Double::try_from(bytes).unwrap();
            assert_eq!(d, Double(123.0));
        }

        #[test]
        fn double_infinito_positivo() {
            let bytes = ",inf\r\n".as_bytes();
            let d = Double::try_from(bytes).unwrap();
            assert_eq!(d, Double(f64::INFINITY));
        }

        #[test]
        fn double_infinito_negativo() {
            let bytes = ",-inf\r\n".as_bytes();
            let d = Double::try_from(bytes).unwrap();
            assert_eq!(d, Double(f64::NEG_INFINITY));
        }

        #[test]
        fn double_nan() {
            let bytes = ",nan\r\n".as_bytes();
            let d = Double::try_from(bytes).unwrap();
            assert!(d.0.is_nan());
        }
    }
}
