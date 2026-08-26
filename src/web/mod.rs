//! HTTP-laag: axum-router, handlers en askama-templates.
//!
//! De UI wordt serverside gerenderd (askama + HTMX vanaf een lokaal meegeleverd
//! bestand); er is bewust geen node-toolchain en geen aparte frontend-build.
//! Handlers roepen nooit rechtstreeks tag- of bestands-API's aan, maar gaan via
//! [`crate::tags`] en [`crate::fs`].
//!
//! Deze module wordt ingevuld door de webserver-taak van fase 0.
