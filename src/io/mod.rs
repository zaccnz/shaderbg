/*
 * io thread
 * handles reading to and from disk
 *
 * this module also contains the code for parsing arguments and config file
 * TODO: create a thread here for all IO operations.
 */

mod args;
mod config;
pub use args::*;
pub use config::*;
