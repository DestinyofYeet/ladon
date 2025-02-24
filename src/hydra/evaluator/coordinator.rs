pub enum ActionState {
    None,
    Evaluating,
    Decoding,
    Building,
}

pub struct ActionHandle {
    handle: usize,
}

impl ActionHandle {
    fn new(id: usize) -> Self {
        ActionHandle {
            handle: id,
        }
    }
}

pub struct Action {
    id: usize,
    flake_uri: String,
    state: ActionState,
}

impl Action {
    fn new(id: usize, flake_uri: String) -> Self {
        Action {
            id,
            flake_uri,
            state: ActionState::None,
        }
    }

    fn set_state(&mut self, state: ActionState) {
        self.state = state;
    }
}

// async fn coordination_callback();

pub struct Coordinator {
    action_counter: usize,
    actions: Vec<Action>,
}

impl Coordinator {
    pub fn new() -> Self {
        Coordinator {
            action_counter: 0,
            actions: Vec::new(),
        }
    }

    fn new_action_id(&mut self) -> usize {
        let counter = self.action_counter;
        self.action_counter += 1;

        counter
    }

    pub async fn schedule(&mut self, flake_uri: String) -> ActionHandle {
        let action = Action::new(self.new_action_id(), flake_uri);

        let handle = ActionHandle::new(action.id);

        handle
    }
}
