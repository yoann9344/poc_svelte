use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use wasm_bindgen::prelude::*;
use web_sys::{console, HtmlButtonElement as Button, HtmlInputElement as Input};

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

struct State {
    counter: u32,
    callbacks: Vec<Closure<dyn FnMut(web_sys::Event) + 'static>>,
}

impl State {
    fn new(runtime: Rc<RefCell<Runtime>>, id: u32) -> Rc<RefCell<Self>> {
        let state = Rc::new(RefCell::new(State {
            counter: 30,
            callbacks: Vec::with_capacity(2),
        }));
        let incrementor = {
            let r = runtime.clone();
            let s = state.clone();
            move |_| {
                let s = s.clone();
                // console::log_2(&"increment : ".into(), &s.borrow().counter.into());
                s.borrow_mut().counter += 10;
                // console::log_1(&"before lock".into());
                if !r.borrow().dirty_ids.contains(&id) {
                    // console::log_1(&"INSERT DIRTY".into());
                    r.borrow_mut().dirty_ids.insert(id);
                }
                // console::log_2(&"INCREMENTED : ".into(), &s.borrow().counter.into());
            }
        };
        let input_bind_value0 = {
            let s = state.clone();
            move |event: web_sys::Event| {
                let s = s.clone();
                s.borrow_mut().counter = event
                    .target()
                    .unwrap_throw()
                    .dyn_ref::<web_sys::HtmlInputElement>()
                    .unwrap_throw()
                    .value_as_number() as u32;
                // console::log_2(&"CB input : ".into(), &s.borrow().counter.into());
            }
        };
        let state = state.clone();
        state
            .borrow_mut()
            .callbacks
            .push(Closure::new(Box::new(incrementor)));
        state
            .borrow_mut()
            .callbacks
            .push(Closure::new(Box::new(input_bind_value0)));

        state.clone()
    }
}

#[wasm_bindgen]
pub struct DOM {
    id: u32,
    button0: Button,
    input: Input,
    state: Rc<RefCell<State>>,
}

#[wasm_bindgen]
impl DOM {
    fn new(runtime: Rc<RefCell<Runtime>>, parent: web_sys::Element) -> Result<Self, JsValue> {
        let document = document!();
        let id = runtime.borrow().next_key;
        let state = State::new(runtime, id);

        let input: Input = document.create_element("input")?.dyn_into()?;
        let button0: Button = document.create_element("button")?.dyn_into()?;

        input.set_attribute("type", "range")?;
        input.set_attribute("min", "10")?;
        input.set_attribute("max", "90")?;
        input.set_attribute("step", "10")?;
        input.set_value_as_number(state.borrow().counter as f64);
        button0.set_text_content(Some("+"));

        parent.append_child(&input)?;
        parent.append_child(&button0)?;

        add_listener!(button0, "click", state, 0)?;
        add_listener!(input, "change", state, 1)?;
        add_listener!(input, "input", state, 1)?;

        Ok(Self {
            id,
            input,
            button0,
            state,
        })
    }
}

impl Drop for DOM {
    fn drop(&mut self) {
        remove_listener!(self.button0, "click", self.state, 0).unwrap_throw();
        remove_listener!(self.input, "change", self.state, 1).unwrap_throw();
        remove_listener!(self.input, "input", self.state, 1).unwrap_throw();
    }
}

trait DOMExt {
    fn update(&self);
    fn as_any(&self) -> &dyn Any;
}

impl DOMExt for DOM {
    fn update(&self) {
        console::log_1(&"update".into());
        self.input
            .set_value_as_number(self.state.borrow().counter as f64);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct Runtime {
    next_key: u32,
    components: HashMap<u32, Box<dyn DOMExt>>,
    dirty_ids: HashSet<u32>,
}

impl Runtime {
    fn new() -> Self {
        Self {
            next_key: 0,
            dirty_ids: HashSet::new(),
            components: HashMap::new(),
        }
    }
}

#[wasm_bindgen(start)]
pub async fn run() -> Result<(), JsValue> {
    let runtime = Rc::new(RefCell::new(Runtime::new()));
    // let component = runtime_locked.components.values().next().unwrap().as_any().downcast_ref::<DOM>().unwrap_throw();
    // let body = body!();
    // // let _component = DOM::new(runtime, body.into());
    let new_component = DOM::new(runtime.clone(), body!().into()).unwrap_throw();
    runtime
        .borrow_mut()
        .components
        .insert(new_component.id, Box::new(new_component));
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
            runtime.borrow().components.get(id).unwrap_throw().update();
            i += 1;
        });
        runtime.borrow_mut().dirty_ids.clear();
        if i > 100 {
            break;
        }
    }
    Ok(())
}
