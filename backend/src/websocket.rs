use std::{
    io::Error as IoError,
    collections::HashMap,
    sync::{Arc, Mutex},
    net::SocketAddr,
};

use async_std::{
    net::{TcpListener, TcpStream},
    task
};
use async_tungstenite::tungstenite::protocol::Message;
use futures::{
    prelude::*,
    channel::mpsc::{unbounded, UnboundedSender},
    pin_mut,
};
use log::info;
use serde::{Deserialize, Serialize};
use crate::eventuler::Task;


type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
type TaskQueue = Arc<Mutex<Vec<Task>>>;


#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Extend(Vec<Task>),
    Task,
}


impl Request {
    pub fn as_message(&self) -> Message {
        Message::Text(serde_json::to_string(self).unwrap())
    }
}


#[derive(Debug, Serialize)]
enum Response {
    OK,
    BadRequest,
    Task(Task),
    EmptyQueue
}


impl Response {
    pub fn as_message(&self) -> Message {
        Message::Text(serde_json::to_string(self).unwrap())
    }
}


async fn accept_connection(
    peer_map: PeerMap,
    queue: TaskQueue,
    stream: TcpStream,
) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");

    info!("Peer address: {}", addr);

    let ws_stream = async_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    let (tx, rx) = unbounded();

    peer_map.lock().unwrap().insert(addr, tx);

    info!("New WebSocket connection: {}", addr);

    let (write, read) = ws_stream.split();

    let incoming = read.try_for_each(|msg| {

        let response_bad_request = || response_bad_request(&peer_map, &addr);

        let text = match msg {
            Message::Text(text) => text,
            _ => return future::ok(response_bad_request())
        };

        let req = match serde_json::from_str::<Request>(&text) {
            Ok(req) => req,
            Err(_) => return future::ok(response_bad_request())
        };


        let resp = handle_request(req, &queue);

        peer_map.lock().unwrap()
            .get(&addr).unwrap()
            .unbounded_send(resp.as_message()).unwrap();

        future::ok(())
    });

    let outgoing = rx.map(Ok).forward(write);

    pin_mut!(incoming, outgoing);

    future::select(incoming, outgoing).await;

    info!("{} disconnected", &addr);

    peer_map.lock().unwrap().remove(&addr);
}


fn response_bad_request(peer_map: &PeerMap, addr: &SocketAddr) {
    let peer_map = peer_map.lock().unwrap();
    let resp = Response::BadRequest.as_message();
    peer_map.get(&addr).unwrap().unbounded_send(resp).unwrap();
}


fn handle_request(
    req: Request,
    queue: &TaskQueue
) -> Response {
    match req {
        Request::Extend(tasks) => { 
            queue.lock().unwrap().extend(tasks);
            Response::OK
        },
        Request::Task => {
            let mut queue = queue.lock().unwrap();
            match queue.is_empty() {
                true => Response::EmptyQueue,
                false => Response::Task(queue.pop().unwrap())
            }
        }
    }
}


pub async fn start_websocket() -> Result<(), IoError> {
    let addr = "127.0.0.1:3001";

    let try_socket = TcpListener::bind(addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    let peer_map = PeerMap::new(Mutex::new(HashMap::new()));
    let queue = Arc::new(Mutex::new(Vec::new()));

    while let Ok((stream, _)) = listener.accept().await {
        task::spawn(
            accept_connection(
                peer_map.clone(),
                queue.clone(),
                stream
            )
        );
    }

    Ok(())
}
