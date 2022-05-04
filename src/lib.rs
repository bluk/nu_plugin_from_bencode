// Copyright 2022 Bryant Luk
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    rust_2018_idioms,
    missing_docs,
    missing_debug_implementations,
    unused_lifetimes,
    unused_qualifications
)]

use indexmap::IndexMap;
use nu_protocol::{ShellError, Span, Spanned, Value};

mod nu;

/// Converts bencode data to Nu structured values.
#[derive(Debug, Default)]
pub struct FromBencode;

fn convert_bencode_to_value(value: bt_bencode::Value, span: Span) -> Result<Value, ShellError> {
    Ok(match value {
        bt_bencode::Value::Int(num) => match num {
            bt_bencode::value::Number::Signed(signed_num) => Value::Int {
                val: signed_num,
                span,
            },
            bt_bencode::value::Number::Unsigned(unsigned_num) => i64::try_from(unsigned_num)
                .map(|val| Value::Int { val, span })
                .map_err(|_| {
                    ShellError::UnsupportedInput("expected a compatible number".into(), span)
                })?,
        },
        bt_bencode::Value::ByteStr(byte_str) => match String::from_utf8(byte_str.into_vec()) {
            Ok(s) => Value::String { val: s, span },
            Err(err) => Value::Binary {
                val: err.into_bytes(),
                span,
            },
        },
        bt_bencode::Value::List(list) => Value::List {
            vals: list
                .into_iter()
                .map(|val| convert_bencode_to_value(val, span))
                .collect::<Result<Vec<_>, ShellError>>()?,
            span,
        },
        bt_bencode::Value::Dict(dict) => {
            let mut collected = Spanned {
                item: IndexMap::new(),
                span,
            };

            for (key, value) in dict {
                let key = String::from_utf8(key.into_vec()).map_err(|e| {
                    ShellError::UnsupportedInput(
                        format!("Unexpected bencode data {:?}:{:?}", e.into_bytes(), value),
                        span,
                    )
                })?;
                let value = convert_bencode_to_value(value, span)?;
                collected.item.insert(key, value);
            }

            Value::from(collected)
        }
    })
}

/// Converts a byte slice into a [`Value`].
///
/// # Errors
///
/// Returns an error if the input is not valid bencode data.
pub fn from_bytes_to_value(bytes: &[u8], span: Span) -> Result<Value, ShellError> {
    let value = bt_bencode::from_slice(bytes).map_err(|_e| {
        ShellError::CantConvert("bencode data".into(), "binary".into(), span, None)
    })?;
    convert_bencode_to_value(value, span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_decode() -> Result<(), bt_bencode::Error> {
        let bencode_bytes = bt_bencode::to_vec(&bt_bencode::Value::from("hello world"))?;
        assert_eq!(bencode_bytes.len(), 14, "{:?}", bencode_bytes);

        let span = Span::new(0, bencode_bytes.len());
        let nu_value = from_bytes_to_value(&bencode_bytes, span).unwrap();
        let expected = Value::String {
            val: "hello world".to_string(),
            span,
        };
        assert_eq!(nu_value, expected);

        Ok(())
    }
}
