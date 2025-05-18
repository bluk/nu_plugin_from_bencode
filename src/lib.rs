#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    rust_2018_idioms,
    missing_docs,
    missing_debug_implementations,
    unused_lifetimes,
    unused_qualifications
)]

use nu_plugin::Plugin;
use nu_protocol::{Record, ShellError, Span, Value};

use bt_bencode::Value as BVal;

mod nu;

/// Converts between bencode data and Nu structured values.
#[derive(Debug, Default)]
pub struct FromBencodePlugin;

impl Plugin for FromBencodePlugin {
    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(nu::FromBencodeCommand),
            Box::new(nu::ToBencodeCommand),
        ]
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }
}

fn convert_bencode_to_value(value: BVal, internal_span: Span) -> Result<Value, ShellError> {
    Ok(match value {
        BVal::Int(num) => match num {
            bt_bencode::value::Number::Signed(signed_num) => Value::int(signed_num, internal_span),
            bt_bencode::value::Number::Unsigned(unsigned_num) => i64::try_from(unsigned_num)
                .map(|val| Value::Int { val, internal_span })
                .map_err(|_| {
                    ShellError::UnsupportedInput {
                        msg: "expected a compatible number".into(),
                        input: format!("{unsigned_num}"),
                        msg_span: internal_span,
                        // TODO: The span is not correct, but there isn't a way to get the span of a value today.
                        input_span: internal_span,
                    }
                })?,
        },
        BVal::ByteStr(byte_str) => match String::from_utf8(byte_str.into_vec()) {
            Ok(s) => Value::string(s, internal_span),
            Err(err) => Value::binary(err.into_bytes(), internal_span),
        },
        BVal::List(list) => Value::list(
            list.into_iter()
                .map(|val| convert_bencode_to_value(val, internal_span))
                .collect::<Result<Vec<_>, ShellError>>()?,
            internal_span,
        ),
        BVal::Dict(dict) => {
            let mut record = Record::new();
            for (key, value) in dict {
                let key = String::from_utf8(key.into_vec()).map_err(|e| {
                    ShellError::UnsupportedInput {
                        msg: format!("Unexpected bencode data {:?}:{:?}", e.into_bytes(), value),
                        input: "key is not a UTF-8 string".into(),
                        msg_span: internal_span,
                        // TODO: The span is not correct, but there isn't a way to get the span of a value today.
                        input_span: internal_span,
                    }
                })?;
                let value = convert_bencode_to_value(value, internal_span)?;
                record.push(key, value);
            }

            Value::record(record, internal_span)
        }
    })
}

/// Converts a byte slice into a [`Value`].
///
/// # Errors
///
/// Returns an error if the input is not valid bencode data.
pub fn from_bytes_to_value(input: &[u8], head: Span) -> Result<Value, ShellError> {
    let value = bt_bencode::from_slice(input).map_err(|_e| ShellError::CantConvert {
        to_type: "bencode data".into(),
        from_type: "binary".into(),
        span: head,
        help: None,
    })?;
    convert_bencode_to_value(value, head)
}

fn convert_value_to_bencode(nu_val: &Value) -> Result<BVal, ShellError> {
    // Handle just enough to round trip: from bencode |to bencode
    match nu_val {
        Value::Int { val, .. } => Ok(BVal::Int((*val).into())),
        Value::String { val, .. } => Ok(BVal::ByteStr(val.clone().into())),
        Value::Binary { val, .. } => Ok(BVal::ByteStr(val.clone().into())),
        Value::List { vals, .. } => Ok(BVal::List(
            vals.into_iter()
                .map(convert_value_to_bencode)
                .collect::<Result<_, _>>()?,
        )),
        Value::Record { val, .. } => Ok(BVal::Dict(
            val.iter()
                .map(|(k, v)| match convert_value_to_bencode(v) {
                    Ok(v) => Ok((bt_bencode::ByteString::from(k.clone()), v)),
                    Err(e) => Err(e),
                })
                .collect::<Result<_, _>>()?,
        )),
        _ => Err(ShellError::CantConvert {
            to_type: "bencode data".into(),
            from_type: "nu value".into(),
            span: nu_val.span(),
            help: None,
        }),
    }
}

/// Nu value, to bt_bencode value, to vec
pub fn from_value_to_bytes(nu_val: &Value, span: Span) -> Result<Vec<u8>, ShellError> {
    let bt_val = convert_value_to_bencode(nu_val)?;
    Ok(
        bt_bencode::to_vec(&bt_val).map_err(|_e| ShellError::CantConvert {
            to_type: "binary".into(),
            from_type: "bencode data".into(),
            span,
            help: None,
        })?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_decode() -> Result<(), bt_bencode::Error> {
        let bencode_bytes = bt_bencode::to_vec(&BVal::from("hello world"))?;
        assert_eq!(bencode_bytes.len(), 14, "{bencode_bytes:?}");

        let internal_span = Span::new(0, bencode_bytes.len());
        let nu_value = from_bytes_to_value(&bencode_bytes, internal_span).unwrap();
        let expected = Value::String {
            val: "hello world".to_string(),
            internal_span,
        };
        assert_eq!(nu_value, expected);

        Ok(())
    }
}
