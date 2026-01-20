// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Plugin runtime using Wasmtime.

use crate::error::{CkError, PluginError, Result};
use wasmtime::{Engine, Instance, Linker, Module, Store};

use super::abi::PluginCapability;
use super::loader::LoadedPlugin;

/// Plugin runtime for executing WASM plugins.
pub struct PluginRuntime {
    engine: Engine,
    allowed_capabilities: Vec<PluginCapability>,
}

impl PluginRuntime {
    /// Create a new plugin runtime.
    pub fn new() -> Result<Self> {
        let engine = Engine::default();

        Ok(Self {
            engine,
            allowed_capabilities: Vec::new(),
        })
    }

    /// Allow specific capabilities.
    pub fn with_capabilities(mut self, capabilities: Vec<PluginCapability>) -> Self {
        self.allowed_capabilities = capabilities;
        self
    }

    /// Execute a loaded plugin.
    pub fn execute(&self, plugin: &LoadedPlugin) -> Result<PluginInstance> {
        // Check capabilities
        for cap in &plugin.info.manifest.permissions {
            if !self.allowed_capabilities.contains(cap) {
                return Err(CkError::Plugin(PluginError::PermissionDenied {
                    name: plugin.info.name.clone(),
                    permission: format!("{:?}", cap),
                }));
            }
        }

        // Compile the module
        let module = Module::new(&self.engine, &plugin.wasm_bytes).map_err(|e| {
            CkError::Plugin(PluginError::LoadFailed {
                name: plugin.info.name.clone(),
                message: format!("Failed to compile WASM: {}", e),
            })
        })?;

        // Create store and linker
        let mut store = Store::new(&self.engine, PluginState::new());
        let mut linker = Linker::new(&self.engine);

        // Add host functions based on capabilities
        self.setup_host_functions(&mut linker)?;

        // Instantiate
        let instance = linker.instantiate(&mut store, &module).map_err(|e| {
            CkError::Plugin(PluginError::LoadFailed {
                name: plugin.info.name.clone(),
                message: format!("Failed to instantiate: {}", e),
            })
        })?;

        Ok(PluginInstance {
            name: plugin.info.name.clone(),
            _store: store,
            _instance: instance,
        })
    }

    /// Set up host functions for the linker.
    fn setup_host_functions(&self, _linker: &mut Linker<PluginState>) -> Result<()> {
        // Add host functions based on allowed capabilities
        // This is where we'd expose ck functionality to plugins

        // For now, just a placeholder
        // In a real implementation, we'd add functions for:
        // - Reading configuration
        // - Accessing git data
        // - Logging
        // etc.

        Ok(())
    }
}

impl Default for PluginRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create plugin runtime")
    }
}

/// State passed to plugins.
#[derive(Debug, Default)]
pub struct PluginState {
    /// Output buffer.
    #[allow(dead_code)]
    pub output: Vec<String>,
    /// Error buffer.
    #[allow(dead_code)]
    pub errors: Vec<String>,
}

impl PluginState {
    /// Create new plugin state.
    pub fn new() -> Self {
        Self::default()
    }
}

/// An instantiated plugin.
pub struct PluginInstance {
    /// Plugin name.
    pub name: String,
    /// Wasmtime store.
    _store: Store<PluginState>,
    /// Wasmtime instance.
    _instance: Instance,
}

impl PluginInstance {
    /// Call the plugin's validate function.
    pub fn validate(&mut self, _message: &str) -> Result<ValidateResult> {
        // Placeholder - would actually call the WASM function
        Ok(ValidateResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }

    /// Call the plugin's generate function.
    pub fn generate(&mut self, _context: &str) -> Result<Option<String>> {
        // Placeholder - would actually call the WASM function
        Ok(None)
    }
}

/// Result from plugin validation.
#[derive(Debug)]
pub struct ValidateResult {
    /// Whether the message is valid.
    pub valid: bool,
    /// Validation errors.
    pub errors: Vec<String>,
    /// Validation warnings.
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_runtime_new() {
        let runtime = PluginRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_plugin_state() {
        let state = PluginState::new();
        assert!(state.output.is_empty());
        assert!(state.errors.is_empty());
    }
}
