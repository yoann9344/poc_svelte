//FOR <% for (name, dom) in sub_doms { %>
//ITEM mod <%= name %> {
//ITEM use super::*;
//ITEM <%= dom.render_once().unwrap() %>
//ITEM } <% } %>

pub struct Props {
    //FOR <% for (ident, type_) in &props { %>
    //ITEM pub <%= ident %>: <%= type_ %>, <% } %>
}

pub struct DOM {
    pub id: u32,
    pub props: Props,
    //FOR <% for (ident, type_) in &fields { %>
    //ITEM <%= ident %>: <%= type_ %>, <% } %>
    // button0: Button,
    // input: Input,
    state: Rc<RefCell<State>>,
    mounted: bool,
}

impl DOM {
    pub fn from_state(state: Rc<RefCell<State>>, id: u32, props: Props) -> Result<Self, JsValue> {
        let document = document!();

        //FOR <% for statement in &init { %>
        //ITEM <%= statement %> <% } %>

        Ok(Self {
            id,
            //FOR <% for (ident, _) in &fields { %>
            //ITEM <%= ident %>, <% } %>
            // input,
            // button0,
            props,
            state,
            mounted: false,
        })
    }
    //IF <% if props.len() == 0 { %>
    pub fn new(runtime: Rc<RefCell<Runtime>>) -> Result<Self, JsValue> {
        let id = runtime.borrow_mut().next_key();
        let state = State::new(runtime, id);

        DOM::from_state(state, id, Props {})
    }
    //ITEM <% } %>
}

impl Drop for DOM {
    fn drop(&mut self) {
        //FOR <% for statement in &drop { %>
        //ITEM <%= statement %> <% } %>
        // remove_listener!(self.button0, "click", self.state, 0).unwrap_throw();
        // remove_listener!(self.input, "change", self.state, 1).unwrap_throw();
    }
}

impl DOMExt for DOM {
    fn mount(&mut self, target: &web_sys::Element) -> Result<(), JsValue> {
        //FOR <% for statement in &mount { %>
        //ITEM <%= statement %> <% } %>

        if !self.mounted {
            //FOR <% for statement in &mount_mounted { %>
            //ITEM <%= statement %> <% } %>
        }
        self.mounted = true;
        Ok(())
    }
    fn update(&mut self) -> Result<(), JsValue> {
        console::log_1(&"update".into());
        //FOR <% for statement in &update { %>
        //ITEM <%= statement %> <% } %>
        // self.input
        //     .set_value_as_number(self.state.borrow().counter as f64);
        Ok(())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
