pub mod app;

pub mod state;

mod routes;

pub mod models;

mod components;

#[cfg(feature = "ssr")]
pub mod hydracore;

#[cfg(feature = "ssr")]
pub mod config;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
