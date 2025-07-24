use crate::processors::{Processor, ProcessorType};

#[derive(Copy, Clone)]
pub(super) struct Json;

impl Json {
    pub(super) fn new() -> Json {
        Json {}
    }
}

impl Processor for Json {
    fn process(&self, input: &[u8]) -> Vec<u8> {
        vec![]
    }

    fn get_type(&self) -> ProcessorType {
        ProcessorType::JSON
    }
}
