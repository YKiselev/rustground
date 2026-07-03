use std::sync::PoisonError;

use ash::vk;
use raw_window_handle::HandleError;
use thiserror::Error;
use winit::error::{self, OsError};

#[derive(Debug, Error)]
pub enum VkError {
    #[error("Vulkan error code: {0}")]
    VkErrorCode(#[from] vk::Result),
    #[error("Generic error: {0}")]
    GenericError(String),
    #[error("Suitability error: {0}")]
    SuitabilityError(&'static str),
    #[error("Swapchain has changed!")]
    SwapchainChanged,
    #[error("String contained an invalid null byte: {0}")]
    InvalidString(#[from] std::ffi::NulError), 
    #[error("Handle error: {0}")]
    HandleError(#[from] HandleError),
    #[error("Lock is poisoned")]
    LockPoisoned,
    #[error("OS error: {0}")]
    OsError(#[from] OsError)
}

impl<T> From<PoisonError<T>> for VkError {
    fn from(_value: PoisonError<T>) -> Self {
        VkError::LockPoisoned
    }
}

pub fn to_generic<E>(e: E) -> VkError
where
    E: ToString,
{
    VkError::GenericError(e.to_string())
}

pub fn to_suitability(cause: &'static str) -> VkError {
    VkError::SuitabilityError(cause)
}