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
