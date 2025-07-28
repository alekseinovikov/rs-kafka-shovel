use crate::transformer::Transformer;
use boa_engine::property::Attribute;
use boa_engine::{Context, JsString, JsValue, Source};

const TRANSFORM_PROCESS: &str = r#"
    var loaded_object = JSON.parse(input);
    var result = transform(loaded_object);
    JSON.stringify(result);
"#;

pub(super) struct JsTransformer {
    context: Context,
}

impl Transformer for JsTransformer {
    fn transform(&mut self, payload: &[u8]) -> Result<Vec<u8>, String> {
        self.run(payload).map(|result| result.as_bytes().to_vec())
    }
}

impl JsTransformer {
    pub(super) fn new(js_transform_function: &str) -> Result<Self, String> {
        let mut context = Context::default();
        context
            .eval(Source::from_bytes(js_transform_function))
            .map_err(|err| err.to_string())?;

        context
            .eval(Source::from_bytes(
                r#"typeof transform === "function" && transform.length === 1"#,
            ))
            .map(|v| v.as_boolean().unwrap_or(false))
            .map_err(|err| err.to_string())?;

        Ok(Self { context })
    }

    pub(super) fn run(&mut self, payload: &[u8]) -> Result<String, String> {
        let input_data: &str =
            std::str::from_utf8(payload).map_err(|_| "Input string is not valid UTF-8")?;
        self.context
            .register_global_property(
                JsString::from("input"),
                JsValue::String(input_data.into()),
                Attribute::all(),
            )
            .map_err(|err| err.to_string())?;

        let result = self
            .context
            .eval(Source::from_bytes(TRANSFORM_PROCESS))
            .map_err(|err| err.to_string())?;

        match result {
            JsValue::String(str) => Ok(str.to_std_string_escaped()),
            other => Err(format!(
                "Unexpected result from JS script engine: {:?}",
                other
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;
    #[test]
    fn test_a_few_string_calls() {
        let js_code = r#"
        function transform(input) {
            return input.msg;
        }"#;
        let transformer = JsTransformer::new(js_code);
        assert_ok!(&transformer);

        let mut transformer = transformer.unwrap();
        let result = transformer.run(b"{\"msg\":\"Hello from Rust\"}");

        assert_ok!(&result);
        assert_eq!(result.unwrap().as_str(), "\"Hello from Rust\"");

        let result = transformer.run(b"{\"msg\":\"Hello World\"}");
        assert_ok!(&result);
        assert_eq!(result.unwrap().as_str(), "\"Hello World\"");
    }

    #[test]
    fn test_new_json_structures() {
        let js_code = r#"
        function transform(input) {
            return {
                "msg": input.msg,
                "number": input.number,
                "array": input.array,
                "object": input.object
            };
        }"#;

        let transformer = JsTransformer::new(js_code);
        assert_ok!(&transformer);

        let mut transformer = transformer.unwrap();
        let result = transformer.run(b"{\"msg\":\"Hello from Rust\",\"number\":1,\"array\":[1,2,3],\"object\":{\"a\":1,\"b\":2}}");
        assert_ok!(&result);
        assert_eq!(
            result.unwrap().as_str(),
            "{\"msg\":\"Hello from Rust\",\"number\":1,\"array\":[1,2,3],\"object\":{\"a\":1,\"b\":2}}"
        );
    }

    #[test]
    fn test_transforming_one_json_to_another() {
        let js_code = r#"
        function transform(input) {
            return {
                "msg": input.msg,
                "number": input.number,
                "a": input.object.a,
                "b": input.object.b,
                "sub_array": input.array.slice(1, 3)
            };
        }"#;

        let transformer = JsTransformer::new(js_code);
        assert_ok!(&transformer);

        let mut transformer = transformer.unwrap();
        let result = transformer.run(b"{\"msg\":\"Hello from Rust\",\"number\":1,\"array\":[1,2,3,4,5],\"object\":{\"a\":1,\"b\":2}}");
        assert_ok!(&result);
        assert_eq!(
            result.unwrap().as_str(),
            "{\"msg\":\"Hello from Rust\",\"number\":1,\"a\":1,\"b\":2,\"sub_array\":[2,3]}"
        );
    }

    #[test]
    fn test_arithmetic_operations() {
        let js_code = r#"
        function transform(input) {
            return {
                "sum": input.a + input.b,
                "difference": input.a - input.b,
                "product": input.a * input.b,
                "quotient": input.a / input.b,
                "power": Math.pow(input.a, input.b),
                "sqrt": Math.sqrt(input.a),
                "rounded": Math.round(input.c)
            };
        }"#;

        let transformer = JsTransformer::new(js_code);
        assert_ok!(&transformer);

        let mut transformer = transformer.unwrap();
        let result = transformer.run(b"{\"a\":16,\"b\":4,\"c\":3.7}");
        assert_ok!(&result);
        assert_eq!(
            result.unwrap().as_str(),
            "{\"sum\":20,\"difference\":12,\"product\":64,\"quotient\":4,\"power\":65536,\"sqrt\":4,\"rounded\":4}"
        );
    }

    #[test]
    fn test_string_manipulation() {
        let js_code = r#"
        function transform(input) {
            return {
                "uppercase": input.text.toUpperCase(),
                "lowercase": input.text.toLowerCase(),
                "length": input.text.length,
                "substring": input.text.substring(7, 11),
                "replaced": input.text.replace("World", "JavaScript"),
                "trimmed": input.whitespace.trim(),
                "split": input.text.split(" ")
            };
        }"#;

        let transformer = JsTransformer::new(js_code);
        assert_ok!(&transformer);

        let mut transformer = transformer.unwrap();
        let result = transformer.run(b"{\"text\":\"Hello World\",\"whitespace\":\"  spaces  \"}");
        assert_ok!(&result);
        assert_eq!(
            result.unwrap().as_str(),
            "{\"uppercase\":\"HELLO WORLD\",\"lowercase\":\"hello world\",\"length\":11,\"substring\":\"orld\",\"replaced\":\"Hello JavaScript\",\"trimmed\":\"spaces\",\"split\":[\"Hello\",\"World\"]}"
        );
    }

    #[test]
    fn test_array_operations() {
        let js_code = r#"
        function transform(input) {
            return {
                "mapped": input.numbers.map(x => x * 2),
                "filtered": input.numbers.filter(x => x > 3),
                "reduced": input.numbers.reduce((acc, val) => acc + val, 0),
                "sorted": [...input.numbers].sort((a, b) => b - a),
                "includes": input.numbers.includes(3),
                "joined": input.words.join("-"),
                "flattened": input.nested.flat()
            };
        }"#;

        let transformer = JsTransformer::new(js_code);
        assert_ok!(&transformer);

        let mut transformer = transformer.unwrap();
        let result = transformer.run(
            b"{\"numbers\":[1,2,3,4,5],\"words\":[\"hello\",\"world\"],\"nested\":[[1,2],[3,4]]}",
        );
        assert_ok!(&result);
        assert_eq!(
            result.unwrap().as_str(),
            "{\"mapped\":[2,4,6,8,10],\"filtered\":[4,5],\"reduced\":15,\"sorted\":[5,4,3,2,1],\"includes\":true,\"joined\":\"hello-world\",\"flattened\":[1,2,3,4]}"
        );
    }

    #[test]
    fn test_conditional_logic() {
        let js_code = r#"
        function transform(input) {
            let result = {};

            // If-else logic
            if (input.value > 10) {
                result.category = "high";
            } else if (input.value > 5) {
                result.category = "medium";
            } else {
                result.category = "low";
            }

            // Ternary operator
            result.isEven = input.value % 2 === 0 ? true : false;

            // Switch statement
            switch(input.type) {
                case "A":
                    result.typeDescription = "Type A";
                    break;
                case "B":
                    result.typeDescription = "Type B";
                    break;
                default:
                    result.typeDescription = "Unknown Type";
            }

            // Logical operators
            result.logicalAnd = input.a && input.b;
            result.logicalOr = input.a || input.b;
            result.logicalNot = !input.a;

            return result;
        }"#;

        let transformer = JsTransformer::new(js_code);
        assert_ok!(&transformer);

        let mut transformer = transformer.unwrap();
        let result = transformer.run(b"{\"value\":8,\"type\":\"A\",\"a\":true,\"b\":false}");
        assert_ok!(&result);
        assert_eq!(
            result.unwrap().as_str(),
            "{\"category\":\"medium\",\"isEven\":true,\"typeDescription\":\"Type A\",\"logicalAnd\":false,\"logicalOr\":true,\"logicalNot\":false}"
        );
    }

    #[test]
    fn test_error_handling_and_validation() {
        let js_code = r#"
        function transform(input) {
            let result = {};

            // Try-catch for error handling
            try {
                if (!input.required) {
                    throw new Error("Required field is missing");
                }
                result.validationPassed = true;
            } catch (e) {
                result.error = e.message;
                result.validationPassed = false;
            }

            // Data validation
            result.isValidEmail = /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(input.email || "");
            result.isPositiveNumber = input.number > 0;

            // Default values with nullish coalescing
            result.username = input.username ?? "anonymous";

            // Type checking
            result.types = {
                numberType: typeof input.number,
                stringType: typeof input.string,
                booleanType: typeof input.boolean,
                objectType: typeof input.object,
                undefinedType: typeof input.undefined
            };

            return result;
        }"#;

        let transformer = JsTransformer::new(js_code);
        assert_ok!(&transformer);

        // Test with valid data
        let mut transformer = transformer.unwrap();
        let result = transformer.run(b"{\"required\":true,\"email\":\"user@example.com\",\"number\":42,\"string\":\"hello\",\"boolean\":true,\"object\":{},\"username\":\"testuser\"}");
        assert_ok!(&result);
        assert_eq!(
            result.unwrap().as_str(),
            "{\"validationPassed\":true,\"isValidEmail\":true,\"isPositiveNumber\":true,\"username\":\"testuser\",\"types\":{\"numberType\":\"number\",\"stringType\":\"string\",\"booleanType\":\"boolean\",\"objectType\":\"object\",\"undefinedType\":\"undefined\"}}"
        );

        // Test with invalid data
        let result = transformer.run(b"{\"email\":\"invalid\",\"number\":-5}");
        assert_ok!(&result);
        assert_eq!(
            result.unwrap().as_str(),
            "{\"error\":\"Required field is missing\",\"validationPassed\":false,\"isValidEmail\":false,\"isPositiveNumber\":false,\"username\":\"anonymous\",\"types\":{\"numberType\":\"number\",\"stringType\":\"undefined\",\"booleanType\":\"undefined\",\"objectType\":\"undefined\",\"undefinedType\":\"undefined\"}}"
        );
    }
}
