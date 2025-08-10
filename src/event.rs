use crate::Clip;

pub(crate) enum Event {
    ConnectivityChanged(bool),
    NewClip(Clip),
}
