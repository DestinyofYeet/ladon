#[cfg(feature = "ssr")]
use crate::hydracore::DB;

#[cfg(feature = "ssr")]
pub fn is_authed(token: String, db: &DB) -> bool {
    return false;
}
