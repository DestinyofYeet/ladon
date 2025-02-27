use std::sync::Mutex;

pub struct State {
    pub value: Mutex<i32>,
}
