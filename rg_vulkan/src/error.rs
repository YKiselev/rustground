use thiserror::Error;
use vulkanalia::vk;

#[derive(Debug, Error, PartialEq)]
pub enum VkError {
    #[error("Vulkan error code: {0}")]
    VkErrorCode(#[from] vk::ErrorCode),
    #[error("Generic error: {0}")]
    GenericError(String),
    #[error("Suitability error: {0}")]
    SuitabilityError(&'static str),
    #[error("Swapchain has changed!")]
    SwapchainChanged,
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
