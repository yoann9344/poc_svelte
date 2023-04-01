let //ITEM <%= name %>
= {
    let r = runtime.clone();
    let s = state.clone();
    move |event: web_sys::Event| {
        let s = s.clone();
        //FOR <% for stmt in &statements { %>
        //ITEM <%= stmt %> <% } %>
        if !r.borrow().dirty_ids.contains(&id) {
            r.borrow_mut().dirty_ids.insert(id);
        }
        //FOR <% for stmt in &modified_idents { %>
        //ITEM <%= stmt %> <% } %>
    }
};
