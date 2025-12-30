use schemars::schema_for;
use sheet::theme::DevupJson;

fn main() {
    let schema = schema_for!(DevupJson);
    let json = serde_json::to_string_pretty(&schema).unwrap();
    println!("{}", json);
}
