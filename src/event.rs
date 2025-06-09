use mpclipboard_common::Clip;

pub(crate) enum Event {
    ConnectivityChanged(bool),
    NewClip(Clip),
}
