use {
    super::{pydantic, validate, ToolError, ToolResult},
    indexmap::IndexMap,
    pyo3::{
        prelude::*,
        types::{PyCFunction, PyModule, PyModuleMethods, PyString},
    },
    std::{
        ffi::{CStr, CString},
        fmt::{self, Write},
        future::Future,
        mem,
        pin::Pin,
        sync::{Arc, Mutex, PoisonError},
    },
    tokio::{runtime::Handle, task},
};

pub type RunFn = Box<
    dyn FnMut(String) -> Pin<Box<dyn Future<Output = ToolResult<String>> + Send + Sync + 'static>>
        + Send
        + Sync
        + 'static,
>;

/// Internal tool data.
struct ToolData {
    name: String,
    description: String,
    run: RunFn,
}

/// A builder for the tool registry.
pub struct ToolRegistryBuilder {
    registry: IndexMap<String, Arc<Mutex<ToolData>>>,
}

/// A registry of tools.
#[derive(Debug)]
pub struct ToolRegistry {
    registry: IndexMap<String, Arc<Mutex<ToolData>>>,
    pub pydantic_module: Py<PyModule>,
    bridged_names: BridgedNames,
}

impl ToolRegistry {
    /// Create a new builder.
    pub fn builder() -> ToolRegistryBuilder {
        ToolRegistryBuilder {
            registry: IndexMap::new(),
        }
    }
}

impl ToolRegistryBuilder {
    /// Register a tool by the specified name.
    pub fn register<N, D, R>(&mut self, name: N, description: D, run: R) -> ToolResult<()>
    where
        N: Into<String>,
        D: Into<String>,
        R: Into<RunFn>,
    {
        let name = name.into();

        validate::validate_tool_name(&name)?;

        let description = description.into();

        validate::validate_tool_description(&description)?;

        let run = run.into();

        if self.registry.contains_key(&name) {
            Err(ToolError::other(format!(
                "Tool name `{name}` already exists, use a unique name."
            )))
        } else {
            let tool_data = Arc::new(Mutex::new(ToolData {
                name: name.clone(),
                description,
                run,
            }));

            self.registry.insert(name, tool_data);

            Ok(())
        }
    }

    pub fn build(self, module_name: &str) -> ToolResult<ToolRegistry> {
        Python::with_gil(|py| {
            let registry = self.registry;
            let bridge_module_name = format!("{module_name}_bridge");
            let pydantic_file_name = format!("{module_name}.py");

            let mut code = String::from("from langchain_core.tools import tool\n\n");
            let mut bridged_names = BridgedNames::new();
            let mut bridged_functions = Vec::new();

            for tool in registry.values() {
                let tool_to_move = Arc::clone(&tool);
                let tool = tool.lock().unwrap_or_else(PoisonError::into_inner);

                let bridged_name =
                    unsafe { bridged_names.push(format!("bridged_{}", &tool.name))? };

                let bridged_function = PyCFunction::new_closure_bound(
                    py,
                    Some(bridged_name),
                    None,
                    move |args, _kwargs| -> PyResult<Py<PyString>> {
                        let py = args.py();

                        let arg = args.get_item(0)?.extract::<String>()?;
                        let mut tool = tool_to_move.lock().unwrap_or_else(PoisonError::into_inner);

                        task::block_in_place(move || {
                            Handle::current().block_on(async move {
                                (tool.run)(arg).await.map_err(ToolError::other)?;

                                Ok::<_, ToolError>(())
                            })
                        })
                        .unwrap();

                        Ok(PyString::new_bound(py, "nigger").unbind())
                    },
                )
                .map_err(ToolError::other)?;

                bridged_functions.push(bridged_function);

                let pydantic_tool = pydantic::Tool {
                    module_name,
                    name: &tool.name,
                    args: "(arg)",
                    output: "str",
                    documentation: &tool.description,
                };

                write!(&mut code, "{pydantic_tool}").map_err(ToolError::other)?;
            }

            let pydantic_module =
                PyModule::from_code_bound(py, &code, &pydantic_file_name, module_name)
                    .map_err(ToolError::other)?;

            for bridged_function in bridged_functions {
                pydantic_module
                    .add_function(bridged_function)
                    .map_err(ToolError::other)?;
            }

            Ok(ToolRegistry {
                registry,
                pydantic_module: pydantic_module.unbind(),
                bridged_names,
            })
        })
    }
}

#[derive(Debug, Default)]
pub struct BridgedNames {
    inner: Vec<CString>,
}

impl BridgedNames {
    pub fn new() -> Self {
        Self::default()
    }

    pub unsafe fn push(&mut self, name: String) -> ToolResult<&'static CStr> {
        let cstring = CString::new(name).map_err(ToolError::other)?;

        self.inner.push(cstring);

        let cstr = self.inner.last_mut().unwrap().as_c_str();

        Ok(mem::transmute::<&CStr, &'static CStr>(cstr))
    }
}

impl fmt::Debug for ToolData {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ToolData")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("run", &(&self.run as *const Box<_>))
            .finish()
    }
}
