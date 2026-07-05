mod app;
mod instance_lock;
mod ipc_client;
mod ipc_worker;
mod symbols;

use iced_layershell::reexport::{Anchor, KeyboardInteractivity};
use iced_layershell::settings::{LayerShellSettings, Settings};

use app::AppState;
use instance_lock::LockOutcome;

fn main() -> iced_layershell::Result {
    // Mantido no escopo de `main` até o fim: remove o lock file no `Drop`
    // quando `run()` retornar (fechamento normal via `iced::exit()`).
    let _guard = match instance_lock::acquire_or_signal_existing() {
        Ok(LockOutcome::SignaledExisting) => return Ok(()),
        Ok(LockOutcome::Acquired(guard)) => Some(guard),
        Err(err) => {
            eprintln!("copied-gui: falha no lock de instância, seguindo sem ele: {err}");
            None
        }
    };

    let settings = Settings {
        layer_settings: LayerShellSettings {
            anchor: Anchor::empty(),
            exclusive_zone: 0,
            size: Some((420, 480)),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..LayerShellSettings::default()
        },
        ..Settings::default()
    };

    iced_layershell::application(AppState::boot, "copied-gui", app::update, app::view)
        .subscription(app::subscription)
        .theme(|_state: &AppState| theme())
        .settings(settings)
        .run()
}

fn theme() -> iced::Theme {
    let palette = iced::theme::Palette {
        primary: iced::Color::from_rgb8(0x20, 0x7f, 0x99),
        ..iced::theme::Palette::DARK
    };
    iced::Theme::custom("copied-gui-dark".to_string(), palette)
}
