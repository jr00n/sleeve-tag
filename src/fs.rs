//! Padafhandeling: de enige plek waar een door de gebruiker aangeleverd pad naar
//! een filesystem-pad wordt vertaald.
//!
//! Elk binnenkomend pad wordt gecanonicaliseerd en gecontroleerd tegen
//! `MUSIC_ROOT`; paden buiten die root en symlinks die eruit wijzen worden
//! geweigerd. Ook de vraag of een bestand bewerkbaar is (`.mp3`/`.flac` én een
//! herkend containerformaat) hoort hier thuis.
//!
//! Binnen deze module wordt `std::fs::` altijd volledig gekwalificeerd
//! geschreven, om verwarring met deze crate-eigen module te voorkomen.
//!
//! Deze module wordt ingevuld door de padafhandelingstaak van fase 1.
