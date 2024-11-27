use crate::ComponentConfig;

use core::fmt::Debug;
use core::time::Duration;
use core::{fmt, iter};

use std::thread;

use anyhow::Context as _;
use clap::Parser as _;
use wasmtime::{InstanceAllocationStrategy, PoolingAllocationConfig};

/// Default max linear memory for a component (256 MiB)
pub const MAX_LINEAR_MEMORY: u64 = 256 * 1024 * 1024;
/// Default max component size (50 MiB)
pub const MAX_COMPONENT_SIZE: u64 = 50 * 1024 * 1024;
/// Default max number of components
pub const MAX_COMPONENTS: u32 = 10_000;

// https://github.com/bytecodealliance/wasmtime/blob/b943666650696f1eb7ff8b217762b58d5ef5779d/src/commands/serve.rs#L641-L656
fn use_pooling_allocator_by_default() -> anyhow::Result<Option<bool>> {
    const BITS_TO_TEST: u32 = 42;
    let mut config = wasmtime::Config::new();
    config.wasm_memory64(true);
    config.static_memory_maximum_size(1 << BITS_TO_TEST);
    let engine = wasmtime::Engine::new(&config)?;
    let mut store = wasmtime::Store::new(&engine, ());
    // NB: the maximum size is in wasm pages to take out the 16-bits of wasm
    // page size here from the maximum size.
    let ty = wasmtime::MemoryType::new64(0, Some(1 << (BITS_TO_TEST - 16)));
    if wasmtime::Memory::new(&mut store, ty).is_ok() {
        Ok(Some(true))
    } else {
        Ok(None)
    }
}

/// [`RuntimeBuilder`] used to configure and build a [Runtime]
#[derive(Clone, Default)]
pub struct RuntimeBuilder {
    engine_config: wasmtime::Config,
    max_components: u32,
    max_component_size: u64,
    max_linear_memory: u64,
    max_execution_time: Duration,
    component_config: ComponentConfig,
    force_pooling_allocator: bool,
}

impl RuntimeBuilder {
    /// Returns a new [`RuntimeBuilder`]
    #[must_use]
    pub fn new() -> Self {
        let mut opts =
            wasmtime_cli_flags::CommonOptions::try_parse_from(iter::empty::<&'static str>())
                .expect("failed to construct common Wasmtime options");
        let mut engine_config = opts
            .config(None, use_pooling_allocator_by_default().unwrap_or(None))
            .expect("failed to construct Wasmtime config");
        engine_config.async_support(true);
        engine_config.wasm_component_model(true);

        Self {
            engine_config,
            max_components: MAX_COMPONENTS,
            // Why so large you ask? Well, python components are chonky, like 35MB for a hello world
            // chonky. So this is pretty big for now.
            max_component_size: MAX_COMPONENT_SIZE,
            max_linear_memory: MAX_LINEAR_MEMORY,
            max_execution_time: Duration::from_secs(10 * 60),
            component_config: ComponentConfig::default(),
            force_pooling_allocator: false,
        }
    }

    /// Set a custom [`ComponentConfig`] to use for all component instances
    #[must_use]
    pub fn component_config(self, component_config: ComponentConfig) -> Self {
        Self {
            component_config,
            ..self
        }
    }

    /// Sets the maximum number of components that can be run simultaneously. Defaults to 10000
    #[must_use]
    pub fn max_components(self, max_components: u32) -> Self {
        Self {
            max_components,
            ..self
        }
    }

    /// Sets the maximum size of a component instance, in bytes. Defaults to 50MB
    #[must_use]
    pub fn max_component_size(self, max_component_size: u64) -> Self {
        Self {
            max_component_size,
            ..self
        }
    }

    /// Sets the maximum amount of linear memory that can be used by all components. Defaults to 10MB
    #[must_use]
    pub fn max_linear_memory(self, max_linear_memory: u64) -> Self {
        Self {
            max_linear_memory,
            ..self
        }
    }

    /// Sets the maximum execution time of a component. Defaults to 10 minutes.
    /// This operates on second precision and value of 1 second is the minimum.
    /// Any value below 1 second will be interpreted as 1 second limit.
    #[must_use]
    pub fn max_execution_time(self, max_execution_time: Duration) -> Self {
        Self {
            max_execution_time: max_execution_time.max(Duration::from_secs(1)),
            ..self
        }
    }

    /// Forces the use of the pooling allocator. This may cause the runtime to fail if there isn't enough memory for the pooling allocator
    #[must_use]
    pub fn force_pooling_allocator(self) -> Self {
        Self {
            force_pooling_allocator: true,
            ..self
        }
    }

    /// Turns this builder into a [`Runtime`]
    ///
    /// # Errors
    ///
    /// Fails if the configuration is not valid
    #[allow(clippy::type_complexity)]
    pub fn build(self) -> anyhow::Result<(Runtime, thread::JoinHandle<Result<(), ()>>)> {
        let engine =
            wasmtime::Engine::new(&self.engine_config).context("failed to construct engine")?;
        Ok((
            Runtime {
                engine,
                component_config: self.component_config,
                max_execution_time: self.max_execution_time,
            },
            thread::spawn(|| Ok(())),
        ))
    }
}

impl TryFrom<RuntimeBuilder> for (Runtime, thread::JoinHandle<Result<(), ()>>) {
    type Error = anyhow::Error;

    fn try_from(builder: RuntimeBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}

/// Shared wasmCloud runtime
#[derive(Clone)]
pub struct Runtime {
    pub(crate) engine: wasmtime::Engine,
    pub(crate) component_config: ComponentConfig,
    pub(crate) max_execution_time: Duration,
}

impl Debug for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Runtime")
            .field("component_config", &self.component_config)
            .field("runtime", &"wasmtime")
            .field("max_execution_time", &"max_execution_time")
            .finish_non_exhaustive()
    }
}

impl Runtime {
    /// Returns a new [`Runtime`] configured with defaults
    ///
    /// # Errors
    ///
    /// Returns an error if the default configuration is invalid
    #[allow(clippy::type_complexity)]
    pub fn new() -> anyhow::Result<(Self, thread::JoinHandle<Result<(), ()>>)> {
        Self::builder().try_into()
    }

    /// Returns a new [`RuntimeBuilder`], which can be used to configure and build a [Runtime]
    #[must_use]
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::new()
    }

    /// [Runtime] version
    #[must_use]
    pub fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
}
