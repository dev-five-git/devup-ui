/// Convert a value to a pixel value
pub fn convert_value(value: &str) -> String {
    let value = value.to_string();
    if let Ok(num) = value.parse::<f64>() {
        let num = num * 4.0;
        return format!("{}px", num);
    }
    value
}
pub fn get_variable_name(property: &str, level: u8) -> String {
    format!("--{}-{}", property, level)
}
pub fn get_variable_class_name(property: &str, level: u8) -> String {
    format!("{}-{}", property, level)
}
