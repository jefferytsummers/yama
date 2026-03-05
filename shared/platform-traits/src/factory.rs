//! Platform factory for runtime creation of platform-specific implementations.
//!
//! This module provides factory functions that detect the current platform
//! and create the appropriate implementation of each trait.

use crate::platform::{detect_platform, Platform, PlatformInfo};
use crate::error::PlatformResult;

/// Platform factory configuration.
#[derive(Debug, Clone, Default)]
pub struct FactoryConfig {
    /// Override the detected platform.
    pub platform_override: Option<Platform>,
    /// Enable debug/verbose logging.
    pub debug: bool,
    /// Prefer software fallback even if hardware is available.
    pub prefer_software: bool,
}

/// Platform factory for creating platform-specific implementations.
///
/// This struct provides a centralized way to create all platform-specific
/// implementations based on runtime detection.
pub struct PlatformFactory {
    info: PlatformInfo,
    config: FactoryConfig,
}

impl PlatformFactory {
    /// Create a new platform factory with auto-detection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            info: detect_platform(),
            config: FactoryConfig::default(),
        }
    }

    /// Create a new platform factory with configuration.
    #[must_use]
    pub fn with_config(config: FactoryConfig) -> Self {
        let mut info = detect_platform();

        // Apply platform override if specified
        if let Some(platform) = config.platform_override {
            info.platform = platform;
        }

        Self { info, config }
    }

    /// Get the detected platform.
    #[must_use]
    pub fn platform(&self) -> Platform {
        self.info.platform
    }

    /// Get full platform information.
    #[must_use]
    pub fn platform_info(&self) -> &PlatformInfo {
        &self.info
    }

    /// Check if running on Apple Silicon.
    #[must_use]
    pub fn is_apple_silicon(&self) -> bool {
        matches!(self.info.platform, Platform::AppleSilicon)
    }

    /// Check if running on NVIDIA Jetson.
    #[must_use]
    pub fn is_jetson(&self) -> bool {
        matches!(self.info.platform, Platform::NvidiaJetson)
    }

    /// Check if CUDA is available.
    #[must_use]
    pub fn has_cuda(&self) -> bool {
        self.info.capabilities.has_cuda
    }

    /// Check if Metal is available.
    #[must_use]
    pub fn has_metal(&self) -> bool {
        self.info.capabilities.has_metal
    }

    /// Check if hardware video decoding is available.
    #[must_use]
    pub fn has_hardware_decode(&self) -> bool {
        self.info.capabilities.has_videotoolbox || self.info.capabilities.has_nvdec
    }

    /// Check if DMA-BUF is available for zero-copy.
    #[must_use]
    pub fn has_dma_buf(&self) -> bool {
        self.info.capabilities.has_dma_buf
    }

    /// Get the recommended renderer backend name.
    #[must_use]
    pub fn recommended_renderer(&self) -> &'static str {
        if self.config.prefer_software {
            return "software";
        }

        match self.info.platform {
            Platform::AppleSilicon | Platform::MacIntel => "metal",
            Platform::NvidiaJetson | Platform::LinuxNvidia => "vulkan",
            Platform::LinuxAmd | Platform::LinuxIntel => "vulkan",
            Platform::WindowsNvidia | Platform::WindowsAmd | Platform::WindowsIntel => "vulkan",
            _ => "software",
        }
    }

    /// Get the recommended video decoder name.
    #[must_use]
    pub fn recommended_decoder(&self) -> &'static str {
        if self.config.prefer_software {
            return "libavcodec";
        }

        match self.info.platform {
            Platform::AppleSilicon | Platform::MacIntel => "videotoolbox",
            Platform::NvidiaJetson => "nvdec_jetson",
            Platform::LinuxNvidia => "nvdec",
            Platform::LinuxAmd | Platform::LinuxIntel => "vaapi",
            _ => "libavcodec",
        }
    }

    /// Get the recommended inference backend name.
    #[must_use]
    pub fn recommended_inference(&self) -> &'static str {
        if self.config.prefer_software {
            return "cpu";
        }

        match self.info.platform {
            Platform::AppleSilicon => "metal",
            Platform::MacIntel => "cpu", // No GPU acceleration on Intel Macs
            Platform::NvidiaJetson | Platform::LinuxNvidia | Platform::WindowsNvidia => "cuda",
            Platform::LinuxAmd | Platform::WindowsAmd => "rocm",
            Platform::LinuxIntel | Platform::WindowsIntel => "openvino",
            _ => "cpu",
        }
    }

    /// Print platform information to log.
    pub fn log_info(&self) {
        tracing::info!("Platform: {}", self.info.platform.name());
        tracing::info!("OS: {} {}", self.info.os, self.info.os_version);
        tracing::info!("CPU: {} ({} cores)", self.info.cpu_model, self.info.cpu_cores);

        if let Some(gpu) = &self.info.gpu_model {
            tracing::info!("GPU: {}", gpu);
        }

        tracing::info!(
            "Memory: {} MB system, {} MB GPU",
            self.info.system_memory / (1024 * 1024),
            self.info.gpu_memory / (1024 * 1024)
        );

        tracing::info!("Capabilities:");
        tracing::info!("  Metal: {}", self.info.capabilities.has_metal);
        tracing::info!("  Vulkan: {}", self.info.capabilities.has_vulkan);
        tracing::info!("  CUDA: {}", self.info.capabilities.has_cuda);
        tracing::info!("  VideoToolbox: {}", self.info.capabilities.has_videotoolbox);
        tracing::info!("  NVDEC: {}", self.info.capabilities.has_nvdec);
        tracing::info!("  VAAPI: {}", self.info.capabilities.has_vaapi);
        tracing::info!("  DRM: {}", self.info.capabilities.has_drm);
        tracing::info!("  DMA-BUF: {}", self.info.capabilities.has_dma_buf);

        tracing::info!("Recommended backends:");
        tracing::info!("  Renderer: {}", self.recommended_renderer());
        tracing::info!("  Decoder: {}", self.recommended_decoder());
        tracing::info!("  Inference: {}", self.recommended_inference());
    }
}

impl Default for PlatformFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to detect and log platform information.
pub fn detect_and_log_platform() -> PlatformFactory {
    let factory = PlatformFactory::new();
    factory.log_info();
    factory
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_creation() {
        let factory = PlatformFactory::new();
        assert!(!factory.platform_info().os.is_empty());
    }

    #[test]
    fn test_recommended_backends() {
        let factory = PlatformFactory::new();
        let renderer = factory.recommended_renderer();
        assert!(!renderer.is_empty());
    }

    #[test]
    fn test_software_override() {
        let config = FactoryConfig {
            prefer_software: true,
            ..Default::default()
        };
        let factory = PlatformFactory::with_config(config);

        assert_eq!(factory.recommended_renderer(), "software");
        assert_eq!(factory.recommended_decoder(), "libavcodec");
        assert_eq!(factory.recommended_inference(), "cpu");
    }
}
