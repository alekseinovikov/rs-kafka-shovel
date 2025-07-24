use boa_engine::property::Attribute;
use boa_engine::{Context, JsString, JsValue, Source};

pub fn run(js: &str, payload: &[u8]) -> Result<String, String> {
    let input_data: &str =
        std::str::from_utf8(payload).map_err(|_| "Input string is not valid UTF-8")?;

    let mut context = Context::default();
    context
        .register_global_property(
            JsString::from("input"),
            JsValue::String(input_data.into()),
            Attribute::READONLY,
        )
        .map_err(|err| err.to_string())?;

    // Parse the source code
    let result = context
        .eval(Source::from_bytes(js))
        .map_err(|err| err.to_string())?;

    match result {
        JsValue::String(str) => Ok(str.to_std_string_escaped()),
        other => Err(format!(
            "JS script must return string, but returned: {:?}",
            other
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_js_mixed_types() {
        let js_code = r#"
                let loaded_object = JSON.parse(input);
                let msg = loaded_object.msg;
                msg
            "#;
        let result = run(js_code, b"{\"msg\": \"Hello from Rust\"}");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "Hello from Rust");
    }
}
