//! Android SDK support for locating SDK stubs
//!
//! This module provides functionality to:
//! - Locate Android SDK using ANDROID_HOME environment variable
//! - Find android.jar for a specific API level
//! - Get path to android-stubs-src.jar for source parsing

use crate::error::{Error, Result};
use std::env;
use std::path::PathBuf;

/// Android SDK configuration
#[derive(Debug, Clone)]
pub struct AndroidSdk {
    /// Path to the Android SDK root
    pub sdk_path: PathBuf,
    /// Android API level (e.g., 33 for Android 13)
    pub api_level: u32,
}

impl AndroidSdk {
    /// Create a new AndroidSdk from ANDROID_HOME environment variable
    pub fn from_env(api_level: u32) -> Result<Self> {
        let sdk_path = env::var("ANDROID_HOME")
            .or_else(|_| env::var("ANDROID_SDK_ROOT"))
            .map_err(|_| {
                Error::AndroidSdk(
                    "ANDROID_HOME or ANDROID_SDK_ROOT environment variable not set".to_string(),
                )
            })?;

        let sdk = Self {
            sdk_path: PathBuf::from(sdk_path),
            api_level,
        };

        // Validate SDK path exists
        if !sdk.sdk_path.exists() {
            return Err(Error::AndroidSdk(format!(
                "Android SDK path does not exist: {}",
                sdk.sdk_path.display()
            )));
        }

        Ok(sdk)
    }

    /// Create a new AndroidSdk from an explicit path
    pub fn from_path(sdk_path: PathBuf, api_level: u32) -> Result<Self> {
        if !sdk_path.exists() {
            return Err(Error::AndroidSdk(format!(
                "Android SDK path does not exist: {}",
                sdk_path.display()
            )));
        }

        Ok(Self {
            sdk_path,
            api_level,
        })
    }

    /// Get the path to android.jar for the configured API level
    pub fn get_android_jar(&self) -> Result<PathBuf> {
        let jar_path = self
            .sdk_path
            .join("platforms")
            .join(format!("android-{}", self.api_level))
            .join("android.jar");

        if !jar_path.exists() {
            return Err(Error::AndroidSdk(format!(
                "android.jar not found for API level {} at: {}",
                self.api_level,
                jar_path.display()
            )));
        }

        Ok(jar_path)
    }

    /// Get the path to the android-stubs-src.jar for the configured API level
    pub fn get_stubs_src_jar(&self) -> Result<PathBuf> {
        let jar_path = self
            .sdk_path
            .join("platforms")
            .join(format!("android-{}", self.api_level))
            .join("android-stubs-src.jar");

        if !jar_path.exists() {
            return Err(Error::AndroidSdk(format!(
                "android-stubs-src.jar not found for API level {} at: {}. You may need to download the Android SDK Sources for this API level.",
                self.api_level,
                jar_path.display()
            )));
        }

        Ok(jar_path)
    }

    /// Get classpath entries needed for parsing (android.jar and annotation libraries)
    pub fn get_classpath(&self) -> Result<Vec<PathBuf>> {
        let mut classpath = vec![self.get_android_jar()?];

        // Add Android support annotations if available
        let annotations_jar = self.sdk_path.join("tools/support/annotations.jar");
        if annotations_jar.exists() {
            classpath.push(annotations_jar);
        }

        // Try androidx annotations from Maven repository if present
        let potential_annotation_paths = [
            // Modern location in SDK
            self.sdk_path
                .join("extras/m2repository/androidx/annotation/annotation"),
            // Legacy support library location
            self.sdk_path
                .join("extras/android/m2repository/com/android/support/support-annotations"),
        ];

        for base_path in &potential_annotation_paths {
            if base_path.exists() {
                // Find the latest version
                if let Ok(entries) = std::fs::read_dir(base_path) {
                    let mut versions: Vec<PathBuf> = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .map(|e| e.path())
                        .collect();

                    versions.sort();

                    if let Some(latest_version) = versions.last() {
                        // Look for the JAR file in the version directory
                        if let Ok(jar_entries) = std::fs::read_dir(latest_version) {
                            for entry in jar_entries.filter_map(|e| e.ok()) {
                                let path = entry.path();
                                if path.extension().and_then(|s| s.to_str()) == Some("jar")
                                    && !path.to_string_lossy().contains("-sources")
                                {
                                    classpath.push(path);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(classpath)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Only run if ANDROID_HOME is set
    fn test_android_sdk_from_env() {
        let sdk = AndroidSdk::from_env(33);
        if let Ok(sdk) = sdk {
            assert!(sdk.sdk_path.exists());
            println!("SDK path: {}", sdk.sdk_path.display());

            if let Ok(jar) = sdk.get_android_jar() {
                assert!(jar.exists());
                println!("android.jar: {}", jar.display());
            }
        }
    }

    #[test]
    #[ignore] // Only run if ANDROID_HOME is set
    fn test_get_stubs_src_jar() {
        let sdk = AndroidSdk::from_env(33);
        if let Ok(sdk) = sdk {
            let jar = sdk.get_stubs_src_jar();
            if let Ok(jar) = jar {
                assert!(jar.exists());
                assert!(jar.to_string_lossy().ends_with("android-stubs-src.jar"));
                println!("android-stubs-src.jar: {}", jar.display());
            }
        }
    }
}
