use gpui_component::ThemeSet;
use schemars::generate::SchemaSettings;

fn main() {
    let settings = SchemaSettings::draft07().for_serialize();
    let generator = settings.into_generator();
    let schema = generator.into_root_schema_for::<ThemeSet>();
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
