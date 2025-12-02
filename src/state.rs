use crate::mrc::{Mrc, MrcWeak, UpgradeError};
use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub struct StateId<T> {
    id: u32,
    _phantom: PhantomData<T>,
}

pub struct StateManager {
    next_id: u32,
    states: HashMap<u32, Box<dyn Any>>,
}

pub enum StateData<T> {
    Hosting(MrcWeak<T>),
    SelfManaged(Mrc<T>),
}

pub struct State<T> {
    id: u32,
    store: StateData<T>,
}

impl<T> PartialEq for State<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        let weak = match &self.store {
            StateData::Hosting(d) => d.clone(),
            StateData::SelfManaged(d) => d.as_weak(),
        };
        Self {
            id: self.id,
            store: StateData::Hosting(weak),
        }
    }
}

impl<T> State<T> {
    pub fn invalid() -> State<T> {
        State {
            id: 0,
            store: StateData::Hosting(MrcWeak::new()),
        }
    }
    pub fn upgrade_mut(&self) -> Result<StateMutRef<'_, T>, UpgradeError> {
        match &self.store {
            StateData::Hosting(d) => {
                let data = d.upgrade()?;
                Ok(StateMutRef {
                    state_data: &self.store,
                    data,
                })
            }
            StateData::SelfManaged(d) => Ok(StateMutRef {
                state_data: &self.store,
                data: d.clone(),
            }),
        }
    }
}

pub struct StateMutRef<'a, T> {
    state_data: &'a StateData<T>,
    data: Mrc<T>,
}

impl<'a, T> Deref for StateMutRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T> DerefMut for StateMutRef<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl StateManager {
    pub fn new() -> StateManager {
        Self {
            next_id: 1,
            states: Default::default(),
        }
    }

    pub fn new_state<T: 'static>(&mut self, state: T) -> State<T> {
        let id = self.next_id;
        self.next_id += 1;

        let data = Mrc::new(state);
        let weak = data.as_weak();
        self.states.insert(id, Box::new(data));
        State {
            id,
            store: StateData::Hosting(weak),
        }
    }

    pub fn new_self_managed_state<T: 'static>(&mut self, state: T) -> State<T> {
        let id = self.next_id;
        self.next_id += 1;

        let data = Mrc::new(state);
        State {
            id,
            store: StateData::SelfManaged(data),
        }
    }

    pub fn remove_state<T>(&mut self, state: &State<T>) {
        self.states.remove(&state.id);
    }

    pub fn get_state<T: 'static>(&self, id: u32) -> Option<State<T>> {
        if let Some(data) = self.states.get(&id) {
            let raw_data = data.deref();
            if let Some(r) = raw_data.downcast_ref::<Mrc<T>>() {
                return Some(State {
                    id,
                    store: StateData::Hosting(r.as_weak()),
                });
            }
        }
        None
    }
}

#[cfg(test)]
pub mod tests {
    use crate::state::StateManager;

    struct MyState {
        num: u32,
    }
    #[test]
    fn test_upgrade_mut() {
        let mut sm = StateManager::new();
        let my_state = MyState { num: 162534 };
        let state = sm.new_state(my_state);

        {
            let state_mut_ref = state.upgrade_mut().unwrap();
            assert_eq!(162534, state_mut_ref.num);
        }

        sm.remove_state(&state);
        let upgrade_result = state.upgrade_mut();
        assert!(upgrade_result.is_err());
    }
}
