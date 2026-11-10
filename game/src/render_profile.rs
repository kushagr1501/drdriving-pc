//! The browser selects a profile before the GL context and car assets are created.
pub fn lightweight() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        extern "C" { fn dr_mobile_rendering() -> i32; }
        unsafe { dr_mobile_rendering() != 0 }
    }
    #[cfg(not(target_arch = "wasm32"))]
    { std::env::var("DRDRIVING_RENDER_PROFILE").as_deref() == Ok("performance") }
}

pub fn active() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        extern "C" { fn dr_game_active() -> i32; }
        unsafe { dr_game_active() != 0 }
    }
    #[cfg(not(target_arch = "wasm32"))]
    { true }
}
