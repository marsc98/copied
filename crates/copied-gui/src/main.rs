mod app;
mod instance_lock;
mod ipc_client;
mod ipc_worker;

use copied_core::Command;
use ipc_client::IpcClient;

fn main() {
    let mut client = match IpcClient::connect() {
        Ok(client) => client,
        Err(err) => {
            eprintln!("copied-gui: falha conectando ao daemon: {err}");
            std::process::exit(1);
        }
    };

    match client.send(&Command::List) {
        Ok(response) => println!("{response:?}"),
        Err(err) => {
            eprintln!("copied-gui: falha na comunicação com o daemon: {err}");
            std::process::exit(1);
        }
    }
}
