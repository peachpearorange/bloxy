#[cfg(target_arch = "wasm32")]
mod web {
  use {std::cell::{Cell, RefCell},
       wasm_bindgen::{JsCast, JsValue, closure::Closure}};

  thread_local! {
    static PASTED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static CAPTURING: Cell<bool> = const { Cell::new(false) };
    static WIDENING: Cell<bool> = const { Cell::new(false) };
    static RESUMING: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
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

  pub fn capture_on_click(wanted: bool) {
    CAPTURING.with(|capturing| capturing.set(wanted))
  }

  pub fn resume_on(keys: Vec<&'static str>) {
    RESUMING.with(|resuming| *resuming.borrow_mut() = keys)
  }

  fn settled(promise: JsValue) {
    if let Ok(promise) = promise.dyn_into::<js_sys::Promise>() {
      let ignored = Closure::<dyn FnMut(JsValue)>::new(|_: JsValue| ());
      let _ = promise.catch(&ignored);
      ignored.forget()
    }
  }

  fn call(target: &web_sys::Element, method: &str) -> JsValue {
    js_sys::Reflect::get(target, &JsValue::from_str(method))
      .ok()
      .and_then(|function| function.dyn_into::<js_sys::Function>().ok())
      .and_then(|function| function.call0(target).ok())
      .unwrap_or(JsValue::UNDEFINED)
  }

  fn lock(document: &web_sys::Document, canvas: &web_sys::Element) {
    if CAPTURING.with(Cell::get) && document.pointer_lock_element().is_none() {
      settled(call(canvas, "requestPointerLock"))
    }
  }

  fn widen(document: web_sys::Document, canvas: web_sys::Element) {
    match call(&canvas, "requestFullscreen").dyn_into::<js_sys::Promise>() {
      Ok(promise) => {
        WIDENING.with(|widening| widening.set(true));
        let (document_after, canvas_after) = (document.clone(), canvas.clone());
        let widened = Closure::<dyn FnMut(JsValue)>::new(move |_: JsValue| {
          WIDENING.with(|widening| widening.set(false));
          lock(&document_after, &canvas_after)
        });
        let refused = Closure::<dyn FnMut(JsValue)>::new(move |_: JsValue| {
          WIDENING.with(|widening| widening.set(false));
          lock(&document, &canvas)
        });
        let _ = promise.then2(&widened, &refused);
        widened.forget();
        refused.forget()
      }
      Err(_) => lock(&document, &canvas)
    }
  }

  pub fn listen_for_clicks() {
    let pressed =
      Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|event: web_sys::MouseEvent| {
        if event.button() == 0
          && let Some(document) = web_sys::window().and_then(|window| window.document())
          && let Some(canvas) = document.query_selector("canvas").ok().flatten()
        {
          match document.fullscreen_element().is_none() {
            true => widen(document, canvas),
            false => lock(&document, &canvas)
          }
        }
      });
    let released =
      Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|event: web_sys::MouseEvent| {
        if event.button() == 0
          && let Some(document) = web_sys::window().and_then(|window| window.document())
          && let Some(canvas) = document.query_selector("canvas").ok().flatten()
          && !WIDENING.with(Cell::get)
        {
          lock(&document, &canvas)
        }
      });
    web_sys::window().map(|window| {
      window
        .add_event_listener_with_callback_and_bool(
          "pointerdown",
          pressed.as_ref().unchecked_ref(),
          true
        )
        .and_then(|_| {
          window.add_event_listener_with_callback_and_bool(
            "pointerup",
            released.as_ref().unchecked_ref(),
            true
          )
        })
    });
    let typed = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
      |event: web_sys::KeyboardEvent| {
        if !event.repeat()
          && RESUMING.with(|resuming| resuming.borrow().contains(&event.code().as_str()))
          && let Some(document) = web_sys::window().and_then(|window| window.document())
          && let Some(canvas) = document.query_selector("canvas").ok().flatten()
          && document.pointer_lock_element().is_none()
        {
          settled(call(&canvas, "requestPointerLock"))
        }
      }
    );
    web_sys::window().map(|window| {
      window.add_event_listener_with_callback_and_bool(
        "keydown",
        typed.as_ref().unchecked_ref(),
        true
      )
    });
    pressed.forget();
    released.forget();
    typed.forget()
  }

  pub fn release() {
    if let Some(document) = web_sys::window().and_then(|window| window.document())
      && document.pointer_lock_element().is_some()
    {
      document.exit_pointer_lock()
    }
  }

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

  pub fn capture_on_click(_wanted: bool) {}

  pub fn resume_on(_keys: Vec<&'static str>) {}

  pub fn listen_for_clicks() {}

  pub fn release() {}

  pub fn pointer_locked() -> Option<bool> { None }
}

pub use web::*;

pub const CAN_COPY: bool = cfg!(target_arch = "wasm32");

pub const WEB: bool = cfg!(target_arch = "wasm32");
