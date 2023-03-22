struct Test {
    //FOR <% for field in &fields_type { %>
    //ITEM <%= field %> <% } %>
}
fn main() {
    let _ = Test {
        //FOR <% for field in &fields_value { %>
        //ITEM <%= field %> <% } %>
    };
}
