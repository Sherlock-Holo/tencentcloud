use std::fmt::Debug;

use serde::{Deserialize, Serialize};

pub trait Api {
    type Request: Serialize + Debug;
    type Response: for<'a> Deserialize<'a> + Debug;

    const VERSION: &'static str;
    const ACTION: &'static str;
    const SERVICE: &'static str;
    const HOST: &'static str;
}
