//! Binário da API web. Toda a lógica vive na lib (`gedocs_server`) para ser
//! testável por integração (`tests/`).

#[tokio::main]
async fn main() {
    gedocs_server::run().await;
}
