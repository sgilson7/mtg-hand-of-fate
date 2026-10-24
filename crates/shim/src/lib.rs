//! tool_shim: forwards values between the page and `tool_core::session`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core and a conversion: Q16 integers to floats for WebGL, the
//! page's milliseconds to microseconds. `crates/core/tests/boundary.rs`
//! fails if a branch, a loop or the `?` operator appears here.

use tool_core::session::Session;
use wasm_bindgen::prelude::*;

fn floats(v: &[i32]) -> Vec<f32> {
    v.iter().map(|&x| x as f32 / 65536.0).collect()
}

#[wasm_bindgen]
pub struct Game {
    inner: Session,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(bundle: &str, save: &str, seed: u32, query: &str) -> Result<Game, JsError> {
        Session::new(bundle, save, tool_core::session::seed_from(query, seed as u64))
            .map(|inner| Game { inner })
            .map_err(|e| JsError::new(&e))
    }
    pub fn frame(&mut self, now_ms: f64) {
        self.inner.frame((now_ms * 1000.0) as i64)
    }
    pub fn act(&mut self, name: &str, value: &str) -> String {
        self.inner.act_json(name, value)
    }
    pub fn ui(&mut self) -> String {
        self.inner.ui_json()
    }
    pub fn ui_version(&self) -> u32 {
        self.inner.ui_version()
    }
    pub fn commands(&self) -> Vec<u32> {
        self.inner.frame_out.commands.clone()
    }
    pub fn values(&self) -> Vec<f32> {
        floats(&self.inner.frame_out.values)
    }
    pub fn views(&self) -> Vec<f32> {
        floats(&self.inner.frame_out.views)
    }
    pub fn static_values(&self) -> Vec<f32> {
        floats(&self.inner.statics.values)
    }
    pub fn static_version(&self) -> u32 {
        self.inner.statics.version
    }
    pub fn take_save(&mut self) -> String {
        self.inner.take_save()
    }
    pub fn pixels(&self) -> Vec<i32> {
        vec![self.inner.pixels.0, self.inner.pixels.1]
    }
    pub fn pick(&mut self, x: u32, y: u32) -> String {
        self.inner.pick(x as i32, y as i32)
    }
}

#[wasm_bindgen]
pub fn numbers() -> String {
    tool_core::web::numbers()
}

#[wasm_bindgen]
pub fn uniforms() -> Vec<f32> {
    floats(&tool_core::web::uniform_values())
}

#[wasm_bindgen]
pub fn mesh(id: u32) -> Vec<f32> {
    floats(&tool_core::web::mesh_values(id))
}
