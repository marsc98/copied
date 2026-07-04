mod app;
mod ipc_client;

use ipc_client::IpcClient;

fn main() {
    let mut client = match IpcClient::connect() {
        Ok(client) => client,
        Err(err) => {
            eprintln!("daemon não encontrado — inicie o serviço (systemctl --user status copied-daemon): {err}");
            std::process::exit(1);
        }
    };

    let mut terminal = ratatui::init();
    let result = app::App::new().run(&mut terminal, &mut client);
    ratatui::restore();

    if let Err(err) = result {
        eprintln!("copied: erro na TUI: {err}");
        std::process::exit(1);
    }
}
