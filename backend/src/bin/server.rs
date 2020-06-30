use futures::{
    future,
    select, 
};
use eventuler::{start_rest, start_websocket};


#[async_std::main]
async fn main() {
    let _ = env_logger::try_init();

    select! {
        _ = future::ready(start_rest()) => panic!("rest service crashed"),
        _ = future::ready(start_websocket()) => panic!("websocket service crashed")
    };
}
