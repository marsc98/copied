use std::sync::mpsc::{self, Sender};

use futures::channel::mpsc::UnboundedReceiver;

use copied_core::{Command, Response};

use crate::ipc_client::IpcClient;

/// Sobe uma thread nativa dona do `IpcClient` bloqueante; comandos entram por
/// um `mpsc::Sender` padrão, respostas saem por um canal `futures` consumível
/// como `Subscription` no loop reativo do iced.
pub fn spawn() -> (Sender<Command>, UnboundedReceiver<Response>) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
    let (resp_tx, resp_rx) = futures::channel::mpsc::unbounded::<Response>();

    std::thread::spawn(move || {
        let mut client = match IpcClient::connect() {
            Ok(client) => client,
            Err(err) => {
                let _ = resp_tx.unbounded_send(Response::Error {
                    message: format!("falha conectando ao daemon: {err}"),
                });
                return;
            }
        };

        for cmd in cmd_rx {
            let response = match client.send(&cmd) {
                Ok(response) => response,
                Err(err) => Response::Error {
                    message: format!("falha na comunicação com o daemon: {err}"),
                },
            };
            if resp_tx.unbounded_send(response).is_err() {
                break;
            }
        }
    });

    (cmd_tx, resp_rx)
}
