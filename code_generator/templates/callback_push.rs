state
    .borrow_mut()
    .callbacks
    .push(Closure::new(Box::new(incrementor)));
