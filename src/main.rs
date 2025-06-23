mod commands;
mod chat;
mod shell;
mod commands_enum;

#[tokio::main]
async fn main() {
    shell::run().await;
}