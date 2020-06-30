use async_tungstenite::tungstenite::Message;
use futures::prelude::*;
use log::info;
use async_std::{
    net::TcpStream,
    task
};

use eventuler::{Request, Task};


async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:3001";
    info!("Address: {}", addr);

    let stream = TcpStream::connect(addr).await?;

    let mut ws_stream = async_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    loop {
        ws_stream.send(Request::Task.as_message()).await?;

        let msg = ws_stream
            .next()
            .await
            .ok_or_else(|| "didn't receive anything")??;

        if let Message::Text(text) = msg {
            let task: Task = serde_json::from_str(&text)?;
            task.run().await;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    task::block_on(run())
}
