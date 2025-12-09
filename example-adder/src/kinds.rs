pub mod calc {
    amimono::rpc_component! {
        //! A generic calculator service.

        const LABEL: &'static str = "calc";

        /// Adds two numbers
        fn add(a: u64, b: u64) -> u64;

        /// Multiplies two numbers
        fn mul(a: u64, b: u64) -> u64;
    }
}

pub mod adder {
    amimono::rpc_component! {
        const LABEL: &'static str = "adder";

        /// Adds two numbers via the calc service
        fn add(a: u64, b: u64) -> u64;
    }
}

pub mod doubler {
    amimono::rpc_component! {
        const LABEL: &'static str = "doubler";

        /// Doubles two numbers via the calc service
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
