//! The refresh cookie. SameSite=Strict plus an Origin check on refresh keeps
//! CSRF out without a token dance, and the cookie is scoped to the auth routes
//! so nothing else ever sees it.

use axum_extra::extract::cookie::{Cookie, SameSite};
use cookie::time::Duration as CookieDuration;

pub const REFRESH_COOKIE: &str = "leaflet_session";
pub const COOKIE_PATH: &str = "/api/v1/auth";

pub fn refresh_cookie(token: &str, max_age: chrono::TimeDelta, secure: bool) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE, token.to_owned()))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .path(COOKIE_PATH)
        .max_age(CookieDuration::seconds(max_age.num_seconds()))
        .build()
}

/// Expiring by value and max_age keeps every browser happy about logging out.
pub fn cleared_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE, ""))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .path(COOKIE_PATH)
        .max_age(CookieDuration::ZERO)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_cookie_is_http_only_strict_and_scoped() {
        let cookie = refresh_cookie("abc", chrono::TimeDelta::days(30), true);
        let rendered = cookie.to_string();
        assert!(rendered.contains("HttpOnly"));
        assert!(rendered.contains("SameSite=Strict"));
        assert!(rendered.contains("Path=/api/v1/auth"));
        assert!(rendered.contains("Secure"));
        assert!(
            !refresh_cookie("abc", chrono::TimeDelta::days(30), false)
                .to_string()
                .contains("Secure")
        );
    }
}
