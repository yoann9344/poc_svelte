let //ITEM <%= name %>
= {
    let r = runtime.clone();
    let s = state.clone();
    move |event: web_sys::Event| {
        let s = s.clone();
        s.borrow_mut().//ITEM <%= bind_ident.to_string() %>
            = event
            .target()
            .unwrap_throw()
            .dyn_ref::<web_sys::HtmlInputElement>()
            .unwrap_throw()
            //ITEM <% if number_type.is_some() { %>
            .value_as_number() as //ITEM <%= number_type.unwrap().to_string() %>
            ;
            //ITEM <% } else { %>
            .value();
            //ITEM <% } %>
        if !r.borrow().dirty_ids.contains(&id) {
            r.borrow_mut().dirty_ids.insert(id);
        }
    }
};
