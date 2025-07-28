use crate::transformer::{get_transformer, TransformerConfig};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::error;

#[derive(Clone)]
pub(crate) struct TransformRequest {
    pub(crate) payload: Vec<u8>,
}

#[derive(Clone)]
pub(crate) struct TransformResponse {
    pub(crate) payload: Vec<u8>,
}

pub(crate) struct TransformTaskConfig {
    transformer_config: TransformerConfig,
    partition: i32,

    channel_capacity: usize,
}

impl TransformTaskConfig {
    pub(crate) fn with_channel_capacity(
        transformer_config: TransformerConfig,
        partition: i32,
        channel_capacity: usize,
    ) -> Self {
        TransformTaskConfig {
            transformer_config,
            partition,
            channel_capacity,
        }
    }

    pub(crate) fn new(transformer_config: TransformerConfig, partition: i32) -> Self {
        TransformTaskConfig {
            transformer_config,
            partition,
            channel_capacity: 100,
        }
    }
}

pub(crate) struct TransformTask {
    pub(crate) sender: Arc<mpsc::Sender<TransformRequest>>,
    pub(crate) receiver: Arc<Mutex<mpsc::Receiver<Result<TransformResponse, String>>>>,
}

impl TransformTask {
    pub(crate) fn new(config: TransformTaskConfig) -> Self {
        let (request_sender, mut request_receiver) =
            mpsc::channel::<TransformRequest>(config.channel_capacity);
        let (response_sender, response_receiver) =
            mpsc::channel::<Result<TransformResponse, String>>(config.channel_capacity);

        std::thread::spawn(move || {
            let mut transformer = get_transformer(config.transformer_config);
            while let Some(request) = request_receiver.blocking_recv() {
                let response = transformer
                    .transform(request.payload.as_slice())
                    .map(|payload| TransformResponse { payload });

                if let Err(error) = response_sender.blocking_send(response) {
                    error!("Failed to send response: {}", error);
                }
            }
        });

        TransformTask {
            sender: Arc::new(request_sender),
            receiver: Arc::new(Mutex::new(response_receiver)),
        }
    }
}
