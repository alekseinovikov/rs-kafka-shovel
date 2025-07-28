use crate::transformer::task::{TransformRequest, TransformResponse, TransformTaskConfig};
use crate::transformer::{task, TransformerConfig};
use futures::TryStreamExt;
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::error::KafkaError;
use rdkafka::message::BorrowedMessage;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::{ClientConfig, Message};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use task::TransformTask;
use tracing::error;

pub(super) struct TopicProcessorConfig {
    kafka_brokers: String,
    kafka_group_id: String,
    input_topic: String,
    output_topic: String,
    transformer_config: TransformerConfig,
}

pub(super) async fn run(config: TopicProcessorConfig) -> Result<(), String> {
    let mut kafka_config = ClientConfig::new();
    kafka_config
        .set("group.id", &config.kafka_group_id)
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set_log_level(RDKafkaLogLevel::Debug);

    let consumer: StreamConsumer = kafka_config.create().expect("Consumer creation failed");
    let producer: FutureProducer = kafka_config.create().expect("Producer creation failed");

    // Create a simple HashMap to store transformers for each partition
    let mut transformers: HashMap<i32, Arc<TransformTask>> = HashMap::new();

    consumer
        .subscribe(&[&config.input_topic])
        .map_err(|e| format!("Failed to subscribe to topic: {e}"))?;

    loop {
        match consumer.recv().await {
            Ok(borrowed_message) => {
                let owned_message = borrowed_message.detach();
                let key = owned_message.key();
                let key = key.map(|k| k.to_owned());
                let partition = owned_message.partition();
                let output_topic = config.output_topic.clone();
                let payload = owned_message.payload();
                if let Some(payload) = payload {
                    let producer_clone = producer.clone();
                    let task = get_or_create_task(&mut transformers, partition, &config);
                    let mut response_receiver = task.receiver.clone();
                    let sender = &task.sender.clone();

                    tokio::spawn(async move {
                        if let Some(response) = response_receiver.lock().await.recv().await {
                            match response {
                                Ok(payload) => {
                                    let key = key.unwrap_or_default();
                                    let record = FutureRecord::to(output_topic.as_str())
                                        .payload(payload.payload.as_slice())
                                        .key(key.as_slice());

                                    producer_clone.send(record, Duration::from_secs(0)).await;
                                }
                                Err(_) => {}
                            }
                        }
                    });

                    let request = TransformRequest {
                        payload: Vec::from(payload),
                    };
                    if let Err(error) = sender.send(request).await {
                        error!("Failed to send message: {:?}", error);
                    }
                }
            }
            Err(error) => {
                error!("Failed to receive message: {:?}", error);
            }
        }
    }
}

fn get_or_create_task(
    transformers: &mut HashMap<i32, Arc<TransformTask>>,
    partition: i32,
    config: &TopicProcessorConfig,
) -> Arc<TransformTask> {
    let config = config.transformer_config.clone();
    transformers
        .entry(partition)
        .or_insert_with(|| {
            let transform_task_config = TransformTaskConfig::new(config, partition);
            Arc::new(TransformTask::new(transform_task_config))
        })
        .clone()
}
