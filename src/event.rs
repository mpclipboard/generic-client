use mpclipboard_common::Clip;

pub enum Event {
    ConnectivityChanged(bool),
    NewClip(Clip),
}
