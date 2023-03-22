use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use wasm_bindgen::prelude::*;
use web_sys::{console, Element, HtmlButtonElement as Button, HtmlInputElement as Input, Text};

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
    //FOR <% for field in &state_fields_type { %>
    //ITEM <%= field %> <% } %>
    callbacks: Vec<Closure<dyn FnMut(web_sys::Event) + 'static>>,
}

impl State {
    fn new(runtime: Rc<RefCell<Runtime>>, id: u32) -> Rc<RefCell<Self>> {
        //ITEM <%= state_init_block %>
        let state = Rc::new(RefCell::new(State {
            //FOR <% for field in &state_fields_value { %>
            //ITEM <%= field %> <% } %>
            callbacks: Vec::with_capacity(
               //ITEM <%= callbacks.len() %>
            ),
        }));

        //FOR <% for (_, callback) in &callbacks { %>
        //ITEM <%= callback %> <% } %>

        //FOR <% for (callback_name, _) in &callbacks { %>
        state.borrow_mut().callbacks.push(Closure::new(Box::new(
                //ITEM <%= callback_name %>
            )));
        //ITEM <% } %>

        state.clone()
    }
}

#[wasm_bindgen]
pub struct DOM {
    id: u32,
    //FOR <% for (ident, type_) in &dom_state { %>
    //ITEM <%= ident %>: <%= type_ %>, <% } %>
    // button0: Button,
    // input: Input,
    state: Rc<RefCell<State>>,
}

impl DOM {
    pub fn new(runtime: Rc<RefCell<Runtime>>, parent: web_sys::Element) -> Result<Self, JsValue> {
        let document = document!();
        let id = runtime.borrow().next_key;
        let state = State::new(runtime, id);

        //FOR <% for statement in &init { %>
        //ITEM <%= statement %> <% } %>

        // let input: Input = document.create_element("input")?.dyn_into()?;
        // let button0: Button = document.create_element("button")?.dyn_into()?;

        // input.set_attribute("type", "range")?;
        // input.set_attribute("min", "10")?;
        // input.set_attribute("max", "90")?;
        // input.set_attribute("step", "10")?;
        // input.set_value_as_number(state.borrow().counter as f64);
        // let div = document.create_element("div")?;
        // div.append_child(&button0)?;

        // let b_0: Node = document.create_element("b")?.dyn_into()?;
        // b_0.set_text_content(Some("Yop"));
        // let text_node_1 = document.create_text_node("+");
        // button0.append_child(&b_0)?;
        // button0.append_child(&text_node_1)?;

        // parent.append_child(&input)?;
        // parent.append_child(&button0)?;

        // add_listener!(button0, "click", state, 0)?;
        // add_listener!(input, "change", state, 1)?;

        Ok(Self {
            id,
            //FOR <% for (ident, _) in &dom_state { %>
            //ITEM <%= ident %>, <% } %>
            // input,
            // button0,
            state,
        })
    }
}

impl Drop for DOM {
    fn drop(&mut self) {
        //FOR <% for statement in &drop { %>
        //ITEM <%= statement %> <% } %>
        // remove_listener!(self.button0, "click", self.state, 0).unwrap_throw();
        // remove_listener!(self.input, "change", self.state, 1).unwrap_throw();
    }
}

trait DOMExt {
    fn update(&self);
    fn as_any(&self) -> &dyn Any;
}

impl DOMExt for DOM {
    fn update(&self) {
        console::log_1(&"update".into());
        //FOR <% for statement in &update { %>
        //ITEM <%= statement %> <% } %>
        // self.input
        //     .set_value_as_number(self.state.borrow().counter as f64);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct Runtime {
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
