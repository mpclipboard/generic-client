use shared_clipboard_common::Clip;

pub enum Event {
    ConnectivityChanged(bool),
    NewClip(Clip),
}
