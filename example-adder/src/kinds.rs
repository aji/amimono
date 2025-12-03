pub mod calc {
    amimono::rpc_component! {
        const LABEL: &'static str = "calc";

        fn add(a: u64, b: u64) -> u64;
        fn mul(a: u64, b: u64) -> u64;
    }
}

pub mod adder {
    amimono::rpc_component! {
        const LABEL: &'static str = "adder";

        fn add(a: u64, b: u64) -> u64;
    }
}

pub mod doubler {
    amimono::rpc_component! {
        const LABEL: &'static str = "doubler";

        fn double(a: u64) -> u64;
    }
}

pub mod driver {
    pub struct DriverKind;

    impl amimono::component::ComponentKind for DriverKind {
        type Instance = ();

        const LABEL: &'static str = "driver";
        const STORAGE: Option<usize> = Some(0);
    }
}
