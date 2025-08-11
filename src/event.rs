use crate::Clip;

pub enum Event {
    ConnectivityChanged(bool),
    NewClip(Clip),
}
