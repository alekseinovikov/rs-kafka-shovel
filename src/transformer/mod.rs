use crate::transformer::js::JsTransformer;

mod js;
pub(super) mod task;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(super) struct TransformerConfig {
    pub transformer_type: TransformerType,
    pub script: String,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(super) enum TransformerType {
    JSON,
}

pub(super) trait Transformer {
    fn transform(&mut self, payload: &[u8]) -> Result<Vec<u8>, String>;
}

pub(super) fn get_transformer(config: TransformerConfig) -> Box<dyn Transformer> {
    match config.transformer_type {
        TransformerType::JSON => Box::new(
            JsTransformer::new(config.script.as_str()).expect("Failed to create transformer"),
        ),
    }
}
