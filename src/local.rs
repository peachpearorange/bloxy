#[cfg(target_arch = "wasm32")]
mod web {
  use {std::cell::RefCell,
       wasm_bindgen::{JsCast, closure::Closure}};

  thread_local! {
    static PASTED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
  }

  fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|window| window.local_storage().ok().flatten())
  }

  pub fn recall(key: &str) -> Option<String> {
    storage().and_then(|storage| storage.get_item(key).ok().flatten())
  }

  pub fn remember(key: &str, value: &str) {
    storage().map(|storage| storage.set_item(key, value));
  }

  pub fn random_bytes(count: usize) -> Vec<u8> {
    let mut bytes = vec![0; count];
    web_sys::window()
      .and_then(|window| window.crypto().ok())
      .map(|crypto| crypto.get_random_values_with_u8_array(&mut bytes));
    bytes
  }

  pub fn copy(text: &str) {
    web_sys::window().map(|window| window.navigator().clipboard().write_text(text));
  }

  pub fn listen_for_paste() {
    let heard = Closure::<dyn FnMut(web_sys::ClipboardEvent)>::new(
      |event: web_sys::ClipboardEvent| {
        if let Some(text) =
          event.clipboard_data().and_then(|data| data.get_data("text").ok())
        {
          PASTED.with(|pasted| pasted.borrow_mut().push(text))
        }
      }
    );
    web_sys::window().and_then(|window| window.document()).map(|document| {
      document.add_event_listener_with_callback("paste", heard.as_ref().unchecked_ref())
    });
    heard.forget()
  }

  pub fn pasted() -> Vec<String> { PASTED.with(|pasted| pasted.take()) }

  pub fn pointer_locked() -> Option<bool> {
    web_sys::window()
      .and_then(|window| window.document())
      .map(|document| document.pointer_lock_element().is_some())
  }
}

#[cfg(not(target_arch = "wasm32"))]
mod web {
  use std::hash::{BuildHasher, RandomState};

  pub fn recall(_key: &str) -> Option<String> { None }

  pub fn remember(_key: &str, _value: &str) {}

  pub fn random_bytes(count: usize) -> Vec<u8> {
    (0..count)
      .map(|index| {
        RandomState::new().hash_one((index, std::time::SystemTime::now())) as u8
      })
      .collect()
  }

  pub fn copy(_text: &str) {}

  pub fn listen_for_paste() {}

  pub fn pasted() -> Vec<String> { Vec::new() }

  pub fn pointer_locked() -> Option<bool> { None }
}

pub use web::*;

pub const CAN_COPY: bool = cfg!(target_arch = "wasm32");
