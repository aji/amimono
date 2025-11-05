use std::{any::Any, collections::HashMap};

use crate::{Context, RPC};

struct MockFn<C: RPC>(Box<dyn Fn(C::Request) -> C::Response>);

enum Mock {
    Placement(Box<dyn Any>),
    Fn(Box<dyn Any>),
}

pub struct TestContext {
    mocks: HashMap<String, Mock>,
}

impl TestContext {
    pub fn new() -> TestContext {
        TestContext {
            mocks: HashMap::new(),
        }
    }

    pub fn mock<C: RPC>(&mut self, f: impl Fn(C::Request) -> C::Response + 'static) {
        let f: Mock = Mock::Fn(Box::new(MockFn::<C>(Box::new(f))));
        self.mocks.insert(C::LABEL.to_owned(), f);
    }

    pub fn place<C: RPC>(&mut self, rpc: C) {
        let c: Mock = Mock::Placement(Box::new(rpc));
        self.mocks.insert(C::LABEL.to_owned(), c);
    }
}

impl Context for TestContext {
    fn call<C: RPC>(&self, req: C::Request) -> C::Response {
        match self.mocks.get(C::LABEL) {
            Some(Mock::Placement(c)) => (c.downcast_ref::<C>().unwrap()).handle(self, req),
            Some(Mock::Fn(f)) => (f.downcast_ref::<MockFn<C>>().unwrap().0)(req),
            None => panic!("unexpected RPC invocation of {}", C::LABEL),
        }
    }
}
