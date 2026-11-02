//! Agent programs as providers: the CLI a vendor ships, run once per request
//! with the sign-in the user already gave it.

// Nothing calls these until the agent client lands on top of them.
#[allow(dead_code)]
mod args;
#[allow(dead_code)]
mod lookup;
