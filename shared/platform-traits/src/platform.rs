//! Platform detection and capability querying.
//!
//! This module provides runtime platform detection and factory functions
//! for creating platform-specific implementations.

use serde::{Deserialize, Serialize};

/// Platform identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Platform {
    /// Apple Silicon (M1, M2, M3, etc.)
    AppleSilicon,
    /// NVIDIA Jetson (Orin, Xavier, etc.)
    NvidiaJetson,
    /// Generic Linux with NVIDIA GPU
    LinuxNvidia,
    /// Generic Linux with AMD GPU
    LinuxAmd,
    /// Generic Linux with Intel GPU
    LinuxIntel,
    /// Generic Linux (CPU only)
    LinuxGeneric,
    /// macOS with Intel
    MacIntel,
    /// Windows with NVIDIA
    WindowsNvidia,
    /// Windows with AMD
    WindowsAmd,
    /// Windows with Intel
    WindowsIntel,
    /// Unknown/unsupported platform
    Unknown,
}

impl Platform {
    /// Get human-readable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::AppleSilicon => "Apple Silicon",
            Self::NvidiaJetson => "NVIDIA Jetson",
            Self::LinuxNvidia => "Linux (NVIDIA)",
            Self::LinuxAmd => "Linux (AMD)",
            Self::LinuxIntel => "Linux (Intel)",
            Self::LinuxGeneric => "Linux (Generic)",
            Self::MacIntel => "macOS (Intel)",
            Self::WindowsNvidia => "Windows (NVIDIA)",
            Self::WindowsAmd => "Windows (AMD)",
            Self::WindowsIntel => "Windows (Intel)",
            Self::Unknown => "Unknown",
        }
    }

    /// Check if this is an Apple platform.
    #[must_use]
    pub const fn is_apple(&self) -> bool {
        matches!(self, Self::AppleSilicon | Self::MacIntel)
    }

    /// Check if this is an NVIDIA platform.
    #[must_use]
    pub const fn is_nvidia(&self) -> bool {
        matches!(
            self,
            Self::NvidiaJetson | Self::LinuxNvidia | Self::WindowsNvidia
        )
    }

    /// Check if this platform has GPU acceleration.
    #[must_use]
    pub const fn has_gpu(&self) -> bool {
        !matches!(self, Self::LinuxGeneric | Self::Unknown)
    }

    /// Check if this is a Jetson device.
    #[must_use]
    pub const fn is_jetson(&self) -> bool {
        matches!(self, Self::NvidiaJetson)
    }
}

/// Platform capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    /// Whether Metal is available.
    pub has_metal: bool,
    /// Whether Vulkan is available.
    pub has_vulkan: bool,
    /// Whether CUDA is available.
    pub has_cuda: bool,
    /// Whether VideoToolbox is available.
    pub has_videotoolbox: bool,
    /// Whether NVDEC is available.
    pub has_nvdec: bool,
    /// Whether VAAPI is available.
    pub has_vaapi: bool,
    /// Whether DRM/KMS is available.
    pub has_drm: bool,
    /// Whether DMA-BUF is supported.
    pub has_dma_buf: bool,
}

impl Default for PlatformCapabilities {
    fn default() -> Self {
        Self {
            has_metal: false,
            has_vulkan: false,
            has_cuda: false,
            has_videotoolbox: false,
            has_nvdec: false,
            has_vaapi: false,
            has_drm: false,
            has_dma_buf: false,
        }
    }
}

/// Platform information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Detected platform.
    pub platform: Platform,
    /// Operating system.
    pub os: String,
    /// OS version.
    pub os_version: String,
    /// CPU architecture.
    pub arch: String,
    /// CPU model name.
    pub cpu_model: String,
    /// Number of CPU cores.
    pub cpu_cores: u32,
    /// GPU model name (if available).
    pub gpu_model: Option<String>,
    /// GPU memory (bytes, 0 if unknown).
    pub gpu_memory: u64,
    /// Total system memory (bytes).
    pub system_memory: u64,
    /// Platform capabilities.
    pub capabilities: PlatformCapabilities,
}

/// Detect the current platform.
#[must_use]
pub fn detect_platform() -> PlatformInfo {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();

    let (platform, capabilities) = detect_platform_impl(&os, &arch);

    PlatformInfo {
        platform,
        os: os.clone(),
        os_version: get_os_version(),
        arch,
        cpu_model: get_cpu_model(),
        cpu_cores: get_cpu_cores(),
        gpu_model: get_gpu_model(&platform),
        gpu_memory: get_gpu_memory(&platform),
        system_memory: get_system_memory(),
        capabilities,
    }
}

fn detect_platform_impl(os: &str, arch: &str) -> (Platform, PlatformCapabilities) {
    match os {
        "macos" => {
            if arch == "aarch64" {
                (
                    Platform::AppleSilicon,
                    PlatformCapabilities {
                        has_metal: true,
                        has_vulkan: true, // MoltenVK
                        has_videotoolbox: true,
                        has_dma_buf: false, // macOS doesn't use DMA-BUF
                        ..Default::default()
                    },
                )
            } else {
                (
                    Platform::MacIntel,
                    PlatformCapabilities {
                        has_metal: true,
                        has_vulkan: true,
                        has_videotoolbox: true,
                        ..Default::default()
                    },
                )
            }
        }
        "linux" => detect_linux_platform(),
        "windows" => detect_windows_platform(),
        _ => (Platform::Unknown, PlatformCapabilities::default()),
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_platform() -> (Platform, PlatformCapabilities) {
    // Check for Jetson
    if is_jetson_device() {
        return (
            Platform::NvidiaJetson,
            PlatformCapabilities {
                has_vulkan: true,
                has_cuda: true,
                has_nvdec: true,
                has_drm: true,
                has_dma_buf: true,
                ..Default::default()
            },
        );
    }

    // Check for NVIDIA GPU
    if has_nvidia_gpu() {
        return (
            Platform::LinuxNvidia,
            PlatformCapabilities {
                has_vulkan: true,
                has_cuda: true,
                has_nvdec: true,
                has_drm: true,
                has_dma_buf: true,
                ..Default::default()
            },
        );
    }

    // Check for AMD GPU
    if has_amd_gpu() {
        return (
            Platform::LinuxAmd,
            PlatformCapabilities {
                has_vulkan: true,
                has_vaapi: true,
                has_drm: true,
                has_dma_buf: true,
                ..Default::default()
            },
        );
    }

    // Check for Intel GPU
    if has_intel_gpu() {
        return (
            Platform::LinuxIntel,
            PlatformCapabilities {
                has_vulkan: true,
                has_vaapi: true,
                has_drm: true,
                has_dma_buf: true,
                ..Default::default()
            },
        );
    }

    (
        Platform::LinuxGeneric,
        PlatformCapabilities {
            has_drm: std::path::Path::new("/dev/dri").exists(),
            has_dma_buf: true,
            ..Default::default()
        },
    )
}

#[cfg(not(target_os = "linux"))]
fn detect_linux_platform() -> (Platform, PlatformCapabilities) {
    (Platform::LinuxGeneric, PlatformCapabilities::default())
}

#[cfg(target_os = "windows")]
fn detect_windows_platform() -> (Platform, PlatformCapabilities) {
    // Simplified detection - would use Windows APIs in real implementation
    (
        Platform::WindowsNvidia,
        PlatformCapabilities {
            has_vulkan: true,
            has_cuda: true,
            has_nvdec: true,
            ..Default::default()
        },
    )
}

#[cfg(not(target_os = "windows"))]
fn detect_windows_platform() -> (Platform, PlatformCapabilities) {
    (Platform::Unknown, PlatformCapabilities::default())
}

#[cfg(target_os = "linux")]
fn is_jetson_device() -> bool {
    // Check for Jetson-specific files
    std::path::Path::new("/etc/nv_tegra_release").exists()
        || std::path::Path::new("/proc/device-tree/compatible")
            .read_to_string()
            .map(|s| s.contains("nvidia,tegra"))
            .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn is_jetson_device() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn has_nvidia_gpu() -> bool {
    std::path::Path::new("/dev/nvidia0").exists()
        || std::path::Path::new("/proc/driver/nvidia").exists()
}

#[cfg(not(target_os = "linux"))]
fn has_nvidia_gpu() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn has_amd_gpu() -> bool {
    // Check for AMD GPU in DRI devices
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            if let Ok(vendor) = std::fs::read_to_string(entry.path().join("device/vendor")) {
                if vendor.trim() == "0x1002" {
                    // AMD vendor ID
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
fn has_amd_gpu() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn has_intel_gpu() -> bool {
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            if let Ok(vendor) = std::fs::read_to_string(entry.path().join("device/vendor")) {
                if vendor.trim() == "0x8086" {
                    // Intel vendor ID
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
fn has_intel_gpu() -> bool {
    false
}

fn get_os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("VERSION_ID="))
                    .map(|l| l.trim_start_matches("VERSION_ID=").trim_matches('"').to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "unknown".to_string()
    }
}

fn get_cpu_model() -> String {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("model name"))
                    .map(|l| l.split(':').nth(1).unwrap_or("unknown").trim().to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "unknown".to_string()
    }
}

fn get_cpu_cores() -> u32 {
    std::thread::available_parallelism()
        .map(|p| p.get() as u32)
        .unwrap_or(1)
}

fn get_gpu_model(platform: &Platform) -> Option<String> {
    match platform {
        Platform::AppleSilicon => Some("Apple Silicon GPU".to_string()),
        Platform::NvidiaJetson => {
            // Would read from /proc/device-tree/model
            Some("NVIDIA Jetson".to_string())
        }
        Platform::LinuxNvidia => {
            // Would use nvidia-smi
            Some("NVIDIA GPU".to_string())
        }
        _ => None,
    }
}

fn get_gpu_memory(platform: &Platform) -> u64 {
    match platform {
        Platform::AppleSilicon => {
            // Unified memory - use a portion of system memory
            get_system_memory() / 2
        }
        Platform::NvidiaJetson => {
            // Varies by model, would read from sysfs
            8 * 1024 * 1024 * 1024 // 8GB default
        }
        _ => 0,
    }
}

fn get_system_memory() -> u64 {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("MemTotal:"))
                    .and_then(|l| {
                        l.split_whitespace()
                            .nth(1)
                            .and_then(|s| s.parse::<u64>().ok())
                            .map(|kb| kb * 1024)
                    })
            })
            .unwrap_or(0)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platform() {
        let info = detect_platform();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.cpu_cores > 0);
    }

    #[test]
    fn test_platform_properties() {
        assert!(Platform::AppleSilicon.is_apple());
        assert!(Platform::NvidiaJetson.is_nvidia());
        assert!(Platform::NvidiaJetson.is_jetson());
        assert!(Platform::AppleSilicon.has_gpu());
        assert!(!Platform::LinuxGeneric.has_gpu());
    }
}
