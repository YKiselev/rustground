use snafu::Snafu;
use vulkanalia::vk;

#[derive(Debug, Snafu)]
pub enum VkError {
    #[snafu(display("Validation layer requested but not supported."))]
    NoValidationLayer,
    #[snafu(display("Vulkan error code: {code}"))]
    VkErrorCode { code: i32 },
    #[snafu(display("Generic error: {cause}"))]
    GenericError { cause: String },
    #[snafu(display("Suitability error: {cause}"))]
    SuitabilityError { cause: &'static str },
}

impl From<vk::ErrorCode> for VkError {
    fn from(value: vk::ErrorCode) -> Self {
        VkError::VkErrorCode {
            code: value.as_raw(),
        }
    }
}

pub fn to_generic<E>(e: E) -> VkError
where
    E: ToString,
{
    VkError::GenericError {
        cause: e.to_string(),
    }
}

pub fn to_suitability(cause: &'static str) -> VkError {
    VkError::SuitabilityError { cause }
}
