pub mod layer {
    pub mod util {
        /// A no-op middleware.
        #[derive(Default, Clone)]
        pub struct Identity {
            _p: (),
        }

        /// Two middlewares chained together.
        #[derive(Clone)]
        pub struct Stack<Inner, Outer> {
            _inner: Inner,
            _outer: Outer,
        }
    }
}
