mod json;
mod js_engine;

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum ProcessorType {
    JSON,
}

pub trait Processor {
    fn process(&self, input: &[u8]) -> Vec<u8>;
    fn get_type(&self) -> ProcessorType;
}

pub fn get_processor(processor_type: ProcessorType) -> impl Processor {
    match processor_type {
        ProcessorType::JSON => json::Json::new(),
    }
}
