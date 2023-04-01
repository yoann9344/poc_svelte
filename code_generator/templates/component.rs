use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use wasm_bindgen::prelude::*;
use web_sys::{console, HtmlButtonElement as Button, HtmlInputElement as Input, Text};

#[macro_export]
macro_rules! to_num {
    ($cell:ident, $type:ty) => {
        $cell.get().parse::<$type>().unwrap_throw()
    };
}

#[macro_export]
macro_rules! mutable {
    ($value:expr) => {
        Rc::new(RefCell::new($value))
    };
}

#[macro_export]
macro_rules! window {
    () => {
        web_sys::window().expect("no global `window` exists")
    };
}

#[macro_export]
macro_rules! document {
    () => {
        window!()
            .document()
            .expect("should have a document on window")
    };
}

#[macro_export]
macro_rules! body {
    () => {
        document!().body().expect("document should have a body")
    };
}

#[macro_export]
macro_rules! next_tick {
    () => {
        gloo_timers::future::TimeoutFuture::new(0).await;
    };
}

#[macro_export]
macro_rules! callback_ref {
    ($state:ident, $index:expr) => {
        $state.borrow().callbacks[$index].as_ref().unchecked_ref()
    };
}

#[macro_export]
macro_rules! listener {
    (add, $element:ident, $event_type:literal, $state:ident, $index:literal) => {
        $element.add_event_listener_with_callback(
            $event_type,
            $state.borrow().callbacks[$index].as_ref().unchecked_ref(),
        )?;
    };
    (remove, $element:ident, $event_type:literal, $state:ident, $index:literal) => {
        $element.add_event_listener_with_callback(
            $event_type,
            $state.borrow().callbacks[$index].as_ref().unchecked_ref(),
        )?;
    };
}

#[macro_export]
macro_rules! add_listener {
    ($element:expr, $event_type:literal, $state:expr, $index:literal) => {
        $element.add_event_listener_with_callback(
            $event_type,
            $state.borrow().callbacks[$index].as_ref().unchecked_ref(),
        )
    };
}

#[macro_export]
macro_rules! remove_listener {
    ($element:expr, $event_type:literal, $state:expr, $index:literal) => {
        $element.remove_event_listener_with_callback(
            $event_type,
            $state.borrow().callbacks[$index].as_ref().unchecked_ref(),
        )
    };
}

trait DOMExt {
    fn mount(&mut self, parent: &web_sys::Element) -> Result<(), JsValue>;
    fn update(&mut self) -> Result<(), JsValue>;
    fn as_any(&self) -> &dyn Any;
}

//ITEM <%= dom._state.borrow().clone().render_once().unwrap() %>

//ITEM <%= dom.render_once().unwrap() %>

pub struct Runtime {
    key: u32,
    components: HashMap<u32, Rc<RefCell<Box<dyn DOMExt>>>>,
    dirty_ids: HashSet<u32>,
}

impl Runtime {
    fn new() -> Self {
        Self {
            key: 0,
            dirty_ids: HashSet::new(),
            components: HashMap::new(),
        }
    }

    fn next_key(&mut self) -> u32 {
        let key = self.key;
        self.key += 1;
        key
    }
}

#[wasm_bindgen(start)]
pub async fn run() -> Result<(), JsValue> {
    let runtime = Rc::new(RefCell::new(Runtime::new()));
    // let component = runtime_locked.components.values().next().unwrap().as_any().downcast_ref::<DOM>().unwrap_throw();
    // let body = body!();
    // // let _component = DOM::new(runtime, body.into());
    let mut new_component = DOM::new(runtime.clone()).unwrap_throw();
    new_component.mount(body!().unchecked_ref())?;
    runtime.borrow_mut().components.insert(
        new_component.id,
        Rc::new(RefCell::new(Box::new(new_component))),
    );
    let mut i = 0;
    loop {
        // sleep(Duration::from_secs(1));
        next_tick!();

        runtime.borrow().dirty_ids.iter().for_each(|id| {
            console::log_2(
                &"dirty : ".into(),
                &format!("dirtys ids {:?}", runtime.borrow().dirty_ids).into(),
            );
            console::log_2(&"dirty : ".into(), &id.clone().into());
            // console::log_2(
            //     &"Component details : ".into(),
            //     &component.state.counter.into(),
            // );
            runtime
                .borrow()
                .components
                .get(id)
                .unwrap_throw()
                .borrow_mut()
                .update()
                .unwrap_throw();
            i += 1;
        });
        runtime.borrow_mut().dirty_ids.clear();
        // FIXME: TODO: Considere removing this break (block infinite loop during dev)
        if i > 100 {
            console::log_1(&"DEV Break main loop (block infinite loop during dev)".into());
            break;
        }
    }
    Ok(())
}
