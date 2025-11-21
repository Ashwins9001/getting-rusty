use rdkafka::consumer::{Consumer, StreamConsumer}; 
use rdkafka::message::BorrowedMessage;
use rdkafka::ClientConfig;
use tokio::task;
use tokio_stream::StreamExt;

/*
Struct: groups pieces of data together
struct Person {
    name: String,
    age: u32,
}

Enum: general type of many variants and have the ability to implement each variant with its own functionality, can return different variants in an enum and more generally,
functions can return different enums as well so 'matching' uses the general pattern to evaluate things.

enum Shape {
    Circle(f64),          // radius
    Rectangle(f64, f64),  // width, height
    Triangle { base: f64, height: f64 },
}


let s = Shape::Circle(2.0);

match s {
    Shape::Circle(r) => println!("Circle with radius {}", r),
    Shape::Rectangle(w, h) => println!("Rectangle {}x{}", w, h),
    Shape::Triangle { base, height } => println!("Triangle {}x{}", base, height),
}

*/

//Script doesn't own the Kafka message, m, it borrows it and contains a reference to it
//the <`_> return type is a lifetime, it means the message, m, cannot live longer than the Kafka lib owner to prevent invalid access 
//The message contains payload, topic, partition, offset, key, headers, timestamp, other metadata
//BorrowedMessage is a struct
async fn process_message(m: BorrowedMessage<'_>) {

    //m.payload_view::<str>() is a method from BorrowedMessage trait and it returns an Option<Result<&T, ErrorType>>
    //Try to convert bit stream to UTF-8 encoded string and actually returns following type: Option<Result<&str, Utf8Error>>
    //Following definitions apply:
    /*

        enum Option<T> {
            Some(T),
            None,
        }

        enum Result<T, E> {
            Ok(T),   // success, contains value of type T
            Err(E),  // failure, contains error of type E
        }

     */
    //So if valid UTF-8 payload: Some(Ok("hello"))
    //if invalid UTF-8 payload: Some(Err(Utf8Error))
    //if no payload: None

    let payload = match m.payload_view::<str>() {
        //Knowing the above : pattern-match -> Some(Ok(s)) means valid payload make a string
        //Otherwise invalid
        Some(Ok(s)) => s.to_string(),
        _ => "<invalid utf8>".into(),
    };

    println!("Processing message: {}", payload);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

#[tokio::main]
async fn main() {
    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or("localhost:9092".into());
    let topic = std::env::var("KAFKA_TOPIC").unwrap_or("test-topic".into());

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("group.id", "rust-consumer-group")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Consumer creation failed");

    consumer.subscribe(&[&topic]).expect("Failed to subscribe");

    println!("Listening for messages on topic: {}", topic);

    let mut stream = consumer.stream();

    while let Some(message_result) = stream.next().await {
        match message_result {
            Ok(msg) => {
                task::spawn(process_message(msg.detach()));
            }
            Err(e) => eprintln!("Error reading message: {:?}", e),
        }
    }
}
