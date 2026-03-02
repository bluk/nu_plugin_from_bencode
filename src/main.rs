use nu_plugin::JsonSerializer;
use nu_plugin_from_bencode::FromBencodePlugin;

fn main() {
    nu_plugin::serve_plugin(&FromBencodePlugin, JsonSerializer {});
}
