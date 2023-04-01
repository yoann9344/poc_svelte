pub struct State {
    //FOR <% for field in &fields_type { %>
    //ITEM <%= field %> <% } %>
    callbacks: Vec<Closure<dyn FnMut(web_sys::Event) + 'static>>,
    updated_idents: HashSet<String>,
}

impl State {
    fn new(runtime: Rc<RefCell<Runtime>>, id: u32) -> Rc<RefCell<Self>> {
        //ITEM <%= init_block %>
        let state = Rc::new(RefCell::new(State {
            //FOR <% for field in &fields_value { %>
            //ITEM <%= field %> <% } %>
            callbacks: Vec::with_capacity(
               //ITEM <%= callbacks.len() %>
            ),
            updated_idents: HashSet::new(),
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
