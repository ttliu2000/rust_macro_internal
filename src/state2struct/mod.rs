pub mod state2struct;
pub mod state2struct_invoke;
pub mod state_typemapping;

pub use state2struct::*;
pub use state2struct_invoke::*;
pub use state_typemapping::*;

use quote::format_ident;
use syn::{Ident, ItemStruct};

fn get_state_type_name(item:&ItemStruct) -> Ident {
    format_ident!("{}StateEnum", item.ident)
}

fn get_trigger_event_type_name(item:&ItemStruct) -> Ident {
    format_ident!("{}TriggerEvents", item.ident)
}