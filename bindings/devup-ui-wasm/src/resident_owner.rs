//! The sole authority for this binding's physical engine bank.
mod access;
mod bank;
mod checkpoint;
mod decision;
mod error;
mod error_capabilities;
mod error_wiring;
mod forward;
mod host;
mod interpreter;
mod lifecycle;
mod protocol;

use std::sync::{LazyLock, Mutex};

use bank::Bank;
use css::resident_owner::OwnerFault;

pub(crate) use error_capabilities::Transport;
pub(crate) use error_wiring::ERRORS;

static BANK: LazyLock<Mutex<Result<Bank, OwnerFault>>> = LazyLock::new(|| Mutex::new(Bank::new()));

pub(crate) fn check_fault() -> Result<(), OwnerFault> {
    access::Access { store: &BANK }.check_fault()
}

pub(crate) fn export_active() -> bool {
    access::Access { store: &BANK }.export_active()
}

#[cfg(test)]
mod decision_tests;

#[cfg(test)]
mod access_tests;
#[cfg(test)]
mod checkpoint_pair_tests;
#[cfg(test)]
mod checkpoint_tests;
#[cfg(test)]
mod error_cleanup_tests;
#[cfg(test)]
mod error_native_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod native_value_tests;
#[cfg(test)]
mod owner_host_tests;
#[cfg(test)]
mod owner_lifecycle_tests;
#[cfg(test)]
mod owner_native_tests;
#[cfg(test)]
mod owner_plan_tests;
#[cfg(test)]
mod owner_protocol_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod driver_native_tests;
#[cfg(test)]
mod driver_reservation_tests;
#[cfg(test)]
mod driver_surface_tests;
#[cfg(test)]
mod driver_tests;
#[cfg(test)]
mod driver_value_tests;
