use crate::cores::system::routes::Route;
use actix_web::web::ServiceConfig;
use actix_web::{Error, HttpRequest, HttpResponse, web};
use futures_util::StreamExt;

#[derive(Debug, Default)]
pub(crate) struct Ws;

impl Route for Ws {
    fn mount(&self, cfg: &mut ServiceConfig, _prefix: &str) {
        cfg.route("/ws", web::get().to(ws_handler));
    }
}

async fn ws_handler(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;
    // todo: Change WS Handler
    actix_web::rt::spawn(async move {
        println!("WebSocket connection opened");
        while let Some(msg) = msg_stream.next().await {
            match msg {
                Ok(actix_ws::Message::Ping(bytes)) => {
                    println!("Ping received, sending Pong...");
                    if let Err(e) = session.pong(&bytes).await {
                        eprintln!("Gagal kirim Pong: {:?}", e);
                        break;
                    }
                }
                Ok(actix_ws::Message::Text(text)) => {
                    println!("Client sent: {}", text);
                    let a = session.text(format!("Echo: {}", text)).await;
                    if let Err(e) = a {
                        println!("Gagal kirim pesan: {:?}", e);
                    }
                }
                Ok(actix_ws::Message::Close(reason)) => {
                    let _ = session.close(reason).await;
                    break;
                }
                Ok(actix_ws::Message::Binary(reason)) => {
                    println!("Binary message received: {:?}", reason);
                    let _ = session.binary(reason).await;
                    break;
                }
                Ok(actix_ws::Message::Nop) => {
                    println!("Nop message received");
                }
                Err(e) => {
                    eprintln!("Websocket error: {:?}", e);
                    break;
                }
                _ => (),
            }
        }
        println!("WebSocket connection closed");
    });

    // send a response object (Status 101) to client
    Ok(res)
}
