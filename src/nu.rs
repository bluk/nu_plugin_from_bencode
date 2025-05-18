use crate::{from_bytes_to_value, from_value_to_bytes, FromBencodePlugin};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, Type, Value};

pub struct FromBencodeCommand;

impl SimplePluginCommand for FromBencodeCommand {
    type Plugin = FromBencodePlugin;

    fn name(&self) -> &str {
        "from bencode"
    }

    fn description(&self) -> &str {
        "Parse data as bencode and create table."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Binary, Type::table())])
            .category(Category::Formats)
    }

    fn run(
        &self,
        _plugin: &FromBencodePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        from_bencode(call, input)
    }
}

fn from_bencode(call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
    let head = call.head;

    let binary_input = input.as_binary()?;

    if binary_input.is_empty() {
        return Ok(Value::Nothing {
            internal_span: head,
        });
    }

    Ok(from_bytes_to_value(binary_input, head)?)
}

pub struct ToBencodeCommand;

impl SimplePluginCommand for ToBencodeCommand {
    type Plugin = FromBencodePlugin;

    fn name(&self) -> &str {
        "to bencode"
    }

    fn description(&self) -> &str {
        "Bencode nu values"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            //.input_output_types(vec![(Type::record(), Type::Binary)])
            .category(Category::Formats)
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        _call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let span = input.span();
        Ok(Value::binary(from_value_to_bytes(input, span)?, span))
    }
}
