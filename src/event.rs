use crate::clip::Clip;

pub(crate) enum Event {
    ConnectivityChanged(bool),
    NewClip(Clip),
}
